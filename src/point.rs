//! [Point]: a position against an [Origin], the affine side of units: temperatures in K, °C, °F and °R, register
//! formulas (`raw / 256 + 25 °C`), gauge and absolute pressure, angles that wrap at a turn.
//!
//! A [Fixed] is an amount (an interval, a vector): 5 K warmer, 3 ms later. A [Point] is a reading on a scale that
//! starts somewhere: 25 °C, 1013 hPa absolute, a heading. The algebra is that of points and vectors:
//!
//! - point + vector = point, point − vector = point, point − point = vector;
//! - point + point and point · x don't compile;
//! - a point converts to another origin of the same unit and [Reference] exactly, by a factor and an offset worked
//!   out at compile time, or doesn't convert (`None`, never rounded); origins of different references (gauge and
//!   absolute pressure) don't convert at all without the reference's value at run time
//!   ([against](Point::against)).
//!
//! An origin is a zero-size marker ([origin!](crate::origin)) carrying its unit, its exact [Offset] from its reference's zero, an
//! optional modulus (a turn, for angles that wrap) and how it is shown (°C, °F). On the wire a point is exactly its
//! number.

use crate::fixed::Number;
use crate::fixed::format::pad;
use crate::{
    Bounds, Fixed, FixedScale, Magnitude, MaybeNan, OutOfBounds, Repr, Storage, Unbounded, Unit,
    UnitExpr, Unlimited,
};
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
#[cfg(feature = "shrink_wrap")]
use shrink_wrap::prelude::*;

pub use offset::Offset;

mod offset;
pub mod origins;
#[cfg(all(test, feature = "std"))]
mod tests;

/// What an [Origin]'s offset is measured from: [UnitZero] (0 K, vacuum) unless the zero is something only known at
/// run time ([AmbientPressure] for gauge pressure, an encoder's index pulse) or a time scale's (TAI's, POSIX time's, a
/// device's monotonic clock's). Points of origins with different references don't convert into each other at compile
/// time.
pub trait Reference: Copy + Default + 'static {
    /// Who the reference is: `exact_units::UnitZero`.
    const ID: crate::MarkerId<'static>;

    /// What the zero is, `""` for the unit's own zero: `"ambient pressure"`.
    const NAME: &'static str;

    /// How its zero is known: [Exact](ReferenceHow::Exact), [RunTime](ReferenceHow::RunTime) or a
    /// [TimeScale](ReferenceHow::TimeScale). No default: a reference says which it is.
    const HOW: ReferenceHow;

    /// All of it as data, for introspection and the API model.
    const DEF: ReferenceDef = ReferenceDef {
        id: Self::ID,
        name: Self::NAME,
        how: Self::HOW,
    };
}

/// A [Reference] as data: what a client needs to tell whether points of two origins convert, and how.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReferenceDef {
    /// The crate and marker that declare it: [Reference::ID].
    pub id: crate::MarkerId<'static>,
    /// [Reference::NAME].
    pub name: &'static str,
    /// [Reference::HOW].
    pub how: ReferenceHow,
}

/// How a [Reference]'s zero is known.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceHow {
    /// A fixed zero of the unit ([UnitZero]): points against it convert at compile time.
    Exact,
    /// A zero only known at run time ([AmbientPressure], an encoder's index pulse): points reach another reference
    /// only through [Point::against] with its value.
    RunTime,
    /// A time scale's zero: points convert at compile time within the scale, and between scales only as its
    /// [TimeScaleDef] allows.
    TimeScale(TimeScaleDef),
}

/// A time scale as data: how it counts seconds and how its zero is known.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeScaleDef {
    /// Whether every SI second is counted (TAI, GPS) or every day is 86400 s (POSIX time).
    pub counting: TimeCounting,
    /// Whether the zero is an agreed moment (1958-01-01 TAI, 1970-01-01 UTC) or a device's own (its boot).
    pub zero: TimeZero,
}

/// How a time scale counts its seconds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeCounting {
    /// Every SI second, uniformly: TAI and GPS time; a device's monotonic clock nominally (its crystal's rate).
    EverySecond,
    /// UTC as POSIX time counts it: every day is 86400 s, a leap second isn't counted (`time_t`, NTP). The
    /// difference to TAI steps at each leap second and needs a table at run time.
    PosixDays,
}

/// How a time scale's zero is known.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeZero {
    /// An agreed moment, the same for everyone: TAI's 1958-01-01, POSIX time's 1970-01-01 UTC.
    Agreed,
    /// A moment of one clock, known only at run time and different per device and per boot: a device's monotonic
    /// clock. Points reach another scale only through [Point::against] with where it lies there (a clock sync).
    RunTime,
}

/// The unit's own zero: 0 K, 0 Pa (vacuum), the zero of an angle.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct UnitZero;

impl Reference for UnitZero {
    const ID: crate::MarkerId<'static> = crate::MarkerId {
        crate_name: "exact_units",
        name: "UnitZero",
    };

    const NAME: &'static str = "";

    const HOW: ReferenceHow = ReferenceHow::Exact;
}

/// The pressure around the sensor, known only at run time: what gauge pressure is measured against.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AmbientPressure;

impl Reference for AmbientPressure {
    const ID: crate::MarkerId<'static> = crate::MarkerId {
        crate_name: "exact_units",
        name: "AmbientPressure",
    };

    const NAME: &'static str = "ambient pressure";

    const HOW: ReferenceHow = ReferenceHow::RunTime;
}

/// Whether the points of an origin wrap around: [Linear] (temperatures, pressures) or [Modulo] (angles).
pub trait Modulus: Copy + Default + 'static {
    /// The period in the origin's unit, `None` for [Linear].
    const MODULUS: Option<Magnitude>;
}

/// Points that don't wrap.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Linear;

impl Modulus for Linear {
    const MODULUS: Option<Magnitude> = None;
}

/// Points that wrap around every `S` of the unit: `Modulo<One>` for [Turn](crate::units::Turn) (a turn),
/// `Modulo<Rational<360, 1>>` for [Degree](crate::units::Degree). A point's number is kept in `[0, M)` for an
/// unsigned storage and `[-M/2, M/2)` for a signed one, and point − point is the shortest way round, signed.
///
/// A period of exactly `2^BITS` steps (a binary angle, [Bam16](crate::Bam16)) wraps in the storage's own wrapping
/// arithmetic, any other (36000 steps of 0.01°) by `rem_euclid`. A period that isn't a whole number of steps doesn't
/// compile:
///
/// ```compile_fail
/// use exact_units::{Fixed, Point, origins::Angle, prefix::Rational, units::Degree};
/// // 360° is not a whole number of 7° steps
/// let a = Point::<Angle<Degree>, Fixed<Degree, Rational<7, 1>, u16>>::from_raw(1);
/// let _ = a + Fixed::new(1);
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modulo<S>(PhantomData<S>);

impl<S: FixedScale> Modulus for Modulo<S> {
    const MODULUS: Option<Magnitude> = Some(S::SCALE);
}

/// An origin of [Point]s: a zero-size marker, never sent, declared with [origin!](crate::origin).
pub trait Origin: Copy + Default + 'static {
    /// The unit of the points' numbers and of the offset: [Kelvin](crate::units::Kelvin) for every temperature
    /// scale, °F too (its numbers are in steps of 5/9 K).
    type Unit: Unit;

    /// What [OFFSET](Self::OFFSET) is measured from.
    type Reference: Reference;

    /// [Linear] or [Modulo].
    type Modulus: Modulus;

    /// Where the origin lies, from the reference's zero, in [Unit](Self::Unit): °C is 273.15 K.
    const OFFSET: Offset;

    /// Who the origin is: the crate that declares it and the marker's name, `exact_units::Celsius`.
    const ID: crate::MarkerId<'static>;

    /// The marker's name: `"Celsius"`.
    const NAME: &'static str;

    /// The symbol a point is shown with, after the number: `"°C"`.
    const SYMBOL: &'static str;

    /// The step that the shown number counts, in [Unit](Self::Unit): 1 for °C, 5/9 for °F.
    const STEP: Magnitude;

    /// Where the shown number's zero lies, from the reference's zero: the [OFFSET](Self::OFFSET) itself, or that of
    /// the scale it is read on (a register formula `raw / 256 + 25 °C` is shown in °C, from 273.15 K).
    const SHOWN: Offset;

    /// All of it as data, for introspection and the API model.
    const DEF: OriginDef = OriginDef {
        id: Self::ID,
        name: Self::NAME,
        symbol: Self::SYMBOL,
        unit: <Self::Unit as Unit>::EXPR,
        reference: <Self::Reference as Reference>::DEF,
        offset: Self::OFFSET,
        modulus: <Self::Modulus as Modulus>::MODULUS,
        step: Self::STEP,
        shown: Self::SHOWN,
    };
}

/// An [Origin] as data: what a client needs to read and show points of an origin it only knows from the API model.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OriginDef {
    /// The crate and marker that declare it: [Origin::ID].
    pub id: crate::MarkerId<'static>,
    /// The marker's name: `"Celsius"`.
    pub name: &'static str,
    /// As written after a number: `"°C"`.
    pub symbol: &'static str,
    /// The unit of the points' numbers.
    pub unit: &'static UnitExpr,
    /// The [Reference] the offset is measured from: [UnitZero] for the unit's zero.
    pub reference: ReferenceDef,
    /// The origin above the reference, in the unit: [Origin::OFFSET].
    pub offset: Offset,
    /// The period in the unit, `None` when points don't wrap.
    pub modulus: Option<Magnitude>,
    /// [Origin::STEP].
    pub step: Magnitude,
    /// [Origin::SHOWN].
    pub shown: Offset,
}

/// Declares an [Origin] marker. Either from the unit's zero (or another [Reference]):
///
/// ```
/// use exact_units::{origin, Offset, Magnitude, units::Kelvin};
///
/// origin! {
///     /// Degrees Rømer: 0 °Rø is 7.5 °C below 0 °C, a degree is 40/21 K.
///     pub struct Romer { unit: Kelvin, symbol: "°Rø", offset: Offset::ratio(53055, 200), step: Magnitude::ratio(40, 21) }
/// }
/// ```
///
/// `step` is 1, `reference` [UnitZero] and `modulus` [Linear] unless given (in that order, after `offset`). Or
/// from another origin, with its unit, reference, modulus and how it is shown, and an offset added:
///
/// ```
/// use exact_units::{origin, Fixed, Offset, Point, origins::Celsius, prefix::Rational, units::Kelvin};
///
/// origin! {
///     /// The LSM6DS temperature register: raw / 256 + 25 °C.
///     pub struct Lsm6dsTemp { above: Celsius, offset: Offset::int(25) }
/// }
/// let t = Point::<Lsm6dsTemp, Fixed<Kelvin, Rational<1, 256>, i16>>::from_raw(128);
/// assert_eq!(t.to_string(), "25.5 °C");
/// ```
#[macro_export]
macro_rules! origin {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            unit: $unit:ty,
            symbol: $symbol:literal,
            offset: $offset:expr
            $(, step: $step:expr)?
            $(, reference: $reference:ty)?
            $(, modulus: $modulus:ty)?
            $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
        $vis struct $name;

        impl $crate::Origin for $name {
            type Unit = $unit;

            type Reference = $crate::origin!(@or $($reference)? ; $crate::UnitZero);

            type Modulus = $crate::origin!(@or $($modulus)? ; $crate::Linear);

            const OFFSET: $crate::Offset = $offset;

            const ID: $crate::MarkerId<'static> = $crate::MarkerId {
                crate_name: env!("CARGO_CRATE_NAME"),
                name: stringify!($name),
            };

            const NAME: &'static str = stringify!($name);

            const SYMBOL: &'static str = $symbol;

            const STEP: $crate::Magnitude = $crate::origin!(@or_expr $($step)? ; $crate::Magnitude::ONE);

            const SHOWN: $crate::Offset = $offset;
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            above: $parent:ty,
            offset: $offset:expr
            $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
        $vis struct $name;

        impl $crate::Origin for $name {
            type Unit = <$parent as $crate::Origin>::Unit;
            type Reference = <$parent as $crate::Origin>::Reference;
            type Modulus = <$parent as $crate::Origin>::Modulus;

            const OFFSET: $crate::Offset = <$parent as $crate::Origin>::OFFSET.add($offset);

            const ID: $crate::MarkerId<'static> = $crate::MarkerId {
                crate_name: env!("CARGO_CRATE_NAME"),
                name: stringify!($name),
            };

            const NAME: &'static str = stringify!($name);

            const SYMBOL: &'static str = <$parent as $crate::Origin>::SYMBOL;

            const STEP: $crate::Magnitude = <$parent as $crate::Origin>::STEP;

            const SHOWN: $crate::Offset = <$parent as $crate::Origin>::SHOWN;
        }
    };
    (@or $given:ty ; $default:ty) => { $given };
    (@or ; $default:ty) => { $default };
    (@or_expr $given:expr ; $default:expr) => { $given };
    (@or_expr ; $default:expr) => { $default };
}

/// A position against the origin `O`, its number a [Fixed] `F` in `O`'s unit: `Point<Celsius, Fixed<Kelvin, Centi,
/// i16>>` is a temperature in 0.01 °C, the same bytes as the `i16`.
///
/// ```
/// use exact_units::{Fixed, Point, origins::{Celsius, Fahrenheit}, prefix::{Centi, Rational}, units::Kelvin};
///
/// type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
/// type DegF = Point<Fahrenheit, Fixed<Kelvin, Rational<5, 9>, i16>>;
/// let body = DegF::from_raw(98);
/// assert_eq!(body.to_string(), "98 °F");
/// // 98 °F is 36.666.. °C: not a whole number of 0.01 °C
/// assert_eq!(body.to::<Celsius, Centi, i16>(), None);
/// assert_eq!(DegF::from_raw(212).to::<Celsius, Centi, i16>(), Some(DegC::from_raw(10000)));
/// // a vector between two points, a point and a vector
/// let warmer = DegC::from_raw(2500) - DegC::from_raw(2000);
/// assert_eq!(warmer, Fixed::new(500));
/// assert_eq!((DegC::from_raw(2000) + warmer).to_string(), "25 °C");
/// ```
///
/// Two points don't add, and a point doesn't scale:
///
/// ```compile_fail
/// use exact_units::{Fixed, Point, origins::Celsius, prefix::Centi, units::Kelvin};
/// type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
/// let _ = DegC::from_raw(2000) + DegC::from_raw(500);
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, Point, origins::Celsius, prefix::Centi, units::Kelvin};
/// type DegC = Point<Celsius, Fixed<Kelvin, Centi, i16>>;
/// let _ = DegC::from_raw(2000) * 2;
/// ```
///
/// Gauge and absolute pressure have different references: no conversion without the ambient pressure.
///
/// ```compile_fail
/// use exact_units::{Fixed, Point, origins::{Gauge, Vacuum}, prefix::Hecto, units::Pascal};
/// let g = Point::<Gauge, Fixed<Pascal, Hecto, i32>>::from_raw(10);
/// let _ = g.to::<Vacuum, Hecto, i32>();
/// ```
#[cfg_attr(
    feature = "shrink_wrap",
    derive_shrink_wrap(borrowed, owned(feature = "std"), transparent)
)]
#[repr(transparent)]
pub struct Point<O, F> {
    /// The point's number: the amount from the origin.
    coord: F,
    /// The origin, in the type only.
    _origin: PhantomData<O>,
}

/// point − point for points of `O` in steps `P` stored as `T`: a [Fixed] in `O`'s unit, signed for a [Modulo] origin.
pub type PointDiff<O, P, T> =
    Fixed<<O as Origin>::Unit, P, <<O as Origin>::Modulus as PointArith<P, T>>::Diff>;

/// How a [Modulus] adds and subtracts the numbers of points in steps `P` stored as `T`: [Linear] as `T` itself,
/// [Modulo] around its period.
pub trait PointArith<P, T: Storage>: Modulus {
    /// The storage of point − point: `T` for [Linear], the signed type of the same width for [Modulo] (`u16` → `i16`).
    type Diff: Storage;

    /// The number of `p` moved by `v`: panics on overflow in debug builds for [Linear], like `T`'s `+`.
    fn add(p: T, v: T) -> T;

    /// [add](Self::add), `None` on overflow.
    fn checked_add(p: T, v: T) -> Option<T>;

    /// The number of `p` moved back by `v`.
    fn sub(p: T, v: T) -> T;

    /// [sub](Self::sub), `None` on overflow.
    fn checked_sub(p: T, v: T) -> Option<T>;

    /// The vector from `b` to `a`.
    fn diff(a: T, b: T) -> Self::Diff;

    /// [diff](Self::diff), `None` on overflow.
    fn checked_diff(a: T, b: T) -> Option<Self::Diff>;
}

impl<P, T: Storage> PointArith<P, T> for Linear {
    type Diff = T;

    #[inline(always)]
    fn add(p: T, v: T) -> T {
        p.add(v)
    }

    #[inline(always)]
    fn checked_add(p: T, v: T) -> Option<T> {
        p.checked_add(v)
    }

    #[inline(always)]
    fn sub(p: T, v: T) -> T {
        p.sub(v)
    }

    #[inline(always)]
    fn checked_sub(p: T, v: T) -> Option<T> {
        p.checked_sub(v)
    }

    #[inline(always)]
    fn diff(a: T, b: T) -> T {
        a.sub(b)
    }

    #[inline(always)]
    fn checked_diff(a: T, b: T) -> Option<T> {
        a.checked_sub(b)
    }
}

/// The period `S` in steps of `P`, a whole number of at most `2^bits`: a compile error otherwise.
#[expect(
    clippy::cast_possible_wrap,
    reason = "n is checked to be at most 2^BITS, BITS at most 64"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a period that is not a whole number of steps"
)]
const fn period_steps<S: FixedScale, P: FixedScale>(bits: u32) -> i128 {
    let Some((n, d)) = S::SCALE.div(P::SCALE).to_ratio() else {
        panic!("the period of a Modulo origin must be a whole number of the point's steps")
    };
    assert!(
        d == 1,
        "the period of a Modulo origin must be a whole number of the point's steps"
    );
    assert!(
        n >= 2 && n <= 1u128 << bits,
        "the period of a Modulo origin must fit the storage (2 to 2^BITS steps)"
    );
    n as i128
}

/// Modular arithmetic for one integer storage: in the type's own wrapping when the period is `2^BITS` steps (a
/// binary angle), else `rem_euclid` in `$wide`.
macro_rules! modulo_int {
    ($($t:ty, $signed:ty, $wide:ty;)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts and the widened sums differ per storage: an #[expect] is unfulfilled where one is exact"
        )]
        impl<S: FixedScale, P: FixedScale> PointArith<P, $t> for Modulo<S> {
            type Diff = $signed;

            #[inline(always)]
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_sign_loss,
                reason = "m is at most 2^BITS and fits $wide; a value reduced below a period of less than 2^BITS fits $t"
            )]
            #[allow(clippy::arithmetic_side_effects, reason = "two $t add in $wide, which has more bits (not flagged for every storage)")]
            fn add(p: $t, v: $t) -> $t {
                let m = const { period_steps::<S, P>(<$t>::BITS) };
                if m == 1 << <$t>::BITS {
                    p.wrapping_add(v)
                } else {
                    canonical::<$t, $wide>(p as $wide + v as $wide, m as $wide) as $t
                }
            }

            #[inline(always)]
            fn checked_add(p: $t, v: $t) -> Option<$t> {
                Some(<Self as PointArith<P, $t>>::add(p, v))
            }

            #[inline(always)]
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_sign_loss,
                reason = "m is at most 2^BITS and fits $wide; a value reduced below a period of less than 2^BITS fits $t"
            )]
            #[allow(clippy::arithmetic_side_effects, reason = "two $t subtract in $wide, which has more bits (not flagged for every storage)")]
            fn sub(p: $t, v: $t) -> $t {
                let m = const { period_steps::<S, P>(<$t>::BITS) };
                if m == 1 << <$t>::BITS {
                    p.wrapping_sub(v)
                } else {
                    canonical::<$t, $wide>(p as $wide - v as $wide, m as $wide) as $t
                }
            }

            #[inline(always)]
            fn checked_sub(p: $t, v: $t) -> Option<$t> {
                Some(<Self as PointArith<P, $t>>::sub(p, v))
            }

            #[inline(always)]
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap,
                clippy::cast_sign_loss,
                reason = "a period of 2^BITS wraps the difference to $signed in two's complement; a smaller one leaves it in [-m/2, m - m/2), which fits"
            )]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "two $t subtract in $wide, which has more bits; m ≤ 2^BITS fits $wide, and d - m with 0 ≤ d < m"
            )]
            fn diff(a: $t, b: $t) -> $signed {
                let m = const { period_steps::<S, P>(<$t>::BITS) };
                if m == 1 << <$t>::BITS {
                    a.wrapping_sub(b) as $signed
                } else {
                    let m = m as $wide;
                    let d = (a as $wide - b as $wide).rem_euclid(m);
                    (if d >= m - m / 2 { d - m } else { d }) as $signed
                }
            }

            #[inline(always)]
            fn checked_diff(a: $t, b: $t) -> Option<$signed> {
                Some(<Self as PointArith<P, $t>>::diff(a, b))
            }
        }
    )*};
}

modulo_int! {
    u8, i8, i32;
    u16, i16, i32;
    u32, i32, i64;
    u64, i64, i128;
    i8, i8, i32;
    i16, i16, i32;
    i32, i32, i64;
    i64, i64, i128;
}

/// `x` reduced into `[0, m)` for an unsigned `T`, `[-m/2, m - m/2)` for a signed one.
#[inline(always)]
fn canonical<T: Storage, W: Wide>(x: W, m: W) -> W {
    let r = x.rem_euclid(m);
    if T::MIN_INT < 0 && r >= m.minus(m.half()) {
        r.minus(m)
    } else {
        r
    }
}

/// `i32`, `i64` and `i128` for [canonical].
trait Wide: Copy + PartialOrd {
    fn rem_euclid(self, m: Self) -> Self;
    fn minus(self, rhs: Self) -> Self;
    fn half(self) -> Self;
}

macro_rules! wide {
    ($($t:ty)*) => {$(
        impl Wide for $t {
            #[inline(always)]
            fn rem_euclid(self, m: Self) -> Self {
                <$t>::rem_euclid(self, m)
            }

            #[inline(always)]
            #[expect(
                clippy::arithmetic_side_effects,
                reason = "canonical's m - m/2 and r - m, with 0 < m and 0 ≤ r < m"
            )]
            fn minus(self, rhs: Self) -> Self {
                self - rhs
            }

            #[inline(always)]
            fn half(self) -> Self {
                self / 2
            }
        }
    )*};
}

wide!(i32 i64 i128);

macro_rules! modulo_float {
    ($($t:ty)*) => {$(
        #[expect(
            clippy::allow_attributes,
            reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact"
        )]
        impl<S: FixedScale, P: FixedScale> PointArith<P, $t> for Modulo<S> {
            type Diff = $t;

            #[inline(always)]
            #[allow(clippy::cast_possible_truncation, reason = "the period rounded to the storage's float once, as any f32 value is")]
            fn add(p: $t, v: $t) -> $t {
                wrap_float(p + v, const { S::SCALE.div(P::SCALE).to_f64() } as $t)
            }

            #[inline(always)]
            fn checked_add(p: $t, v: $t) -> Option<$t> {
                Some(<Self as PointArith<P, $t>>::add(p, v)).filter(|x| x.is_finite())
            }

            #[inline(always)]
            #[allow(clippy::cast_possible_truncation, reason = "the period rounded to the storage's float once, as any f32 value is")]
            fn sub(p: $t, v: $t) -> $t {
                wrap_float(p - v, const { S::SCALE.div(P::SCALE).to_f64() } as $t)
            }

            #[inline(always)]
            fn checked_sub(p: $t, v: $t) -> Option<$t> {
                Some(<Self as PointArith<P, $t>>::sub(p, v)).filter(|x| x.is_finite())
            }

            #[inline(always)]
            #[allow(clippy::cast_possible_truncation, reason = "the period rounded to the storage's float once, as any f32 value is")]
            fn diff(a: $t, b: $t) -> $t {
                let m = const { S::SCALE.div(P::SCALE).to_f64() } as $t;
                let mut d = (a - b) % m;
                if d < 0.0 {
                    d += m;
                }
                if d >= m / 2.0 { d - m } else { d }
            }

            #[inline(always)]
            fn checked_diff(a: $t, b: $t) -> Option<$t> {
                Some(<Self as PointArith<P, $t>>::diff(a, b)).filter(|x| x.is_finite())
            }
        }
    )*};
}

modulo_float!(f32 f64);

/// `x` in `[0, m)`.
#[inline(always)]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "F is f32 or f64 (modulo_float): float arithmetic, no overflow panic, IEEE semantics"
)]
fn wrap_float<
    F: Copy + PartialOrd + core::ops::Rem<Output = F> + core::ops::Add<Output = F> + Default,
>(
    x: F,
    m: F,
) -> F {
    let r = x % m;
    if r < F::default() { r + m } else { r }
}

impl<O: Origin<Unit = U>, U, P, T: Storage, B: Unlimited<T>> Point<O, Fixed<U, P, T, B>> {
    /// The point whose number is `raw` steps of `P` from the origin, for every bounds marker without limits
    /// ([Unbounded](crate::Unbounded), [Wrapping](crate::Wrapping) as in a [Bam16](crate::Bam16) heading,
    /// [Saturating](crate::Saturating)). Integer storages only, as [Fixed::of]: a float point is made with
    /// [bounded](Self::bounded) or `Point::new(Fixed::try_new(raw)?)`.
    #[inline(always)]
    pub const fn from_raw(raw: T) -> Self {
        Self::new(Fixed::of(raw))
    }
}

/// `bounded(raw)` of a float point, as [Fixed::bounded].
macro_rules! bounded_float {
    ($($ty:ident)*) => {$(
        impl<O: Origin<Unit = U>, U, P, B: Bounds<$ty>> Point<O, Fixed<U, P, $ty, B>> {
            /// The point `raw` steps of `P` from the origin: a compile error in `const` code when `raw` is NaN
            /// (unless `B` is [MaybeNan](crate::MaybeNan)) or outside `B`, a panic at run time.
            #[inline(always)]
            pub const fn bounded(raw: $ty) -> Self {
                Self::new(Fixed::<U, P, $ty, B>::bounded(raw))
            }
        }
    )*};
}

bounded_float!(f32 f64);

impl<O, F> Point<O, F> {
    /// The point `coord` from the origin.
    #[inline(always)]
    pub const fn new(coord: F) -> Self {
        Point {
            coord,
            _origin: PhantomData,
        }
    }
}

impl<O: Origin<Unit = U>, U, P, T: Storage, B: Bounds<T>> Point<O, Fixed<U, P, T, B>> {
    /// The number as a [Fixed]: how far from the origin.
    #[inline(always)]
    pub const fn get(self) -> Fixed<U, P, T, B> {
        self.coord
    }

    /// The number as stored.
    #[inline(always)]
    pub const fn raw(self) -> T {
        self.coord.get()
    }

    /// The point `raw` steps from the origin, if `raw` is within `B`.
    #[inline(always)]
    pub fn try_from_raw(raw: T) -> Result<Self, OutOfBounds> {
        Fixed::try_new(raw).map(Self::new)
    }

    /// The same point without bounds, `None` where it is NaN: how a [MaybeNan] point (`+` of floats) goes back to a
    /// checked one, which has `<` and `==`.
    #[inline(always)]
    pub fn not_nan(self) -> Option<Point<O, Fixed<U, P, T>>> {
        self.coord.not_nan().map(Point::new)
    }

    /// `self > rhs`, and `nan` where `self` is NaN: a [MaybeNan] point compares so, the NaN outcome written at the
    /// call, in one float compare ([Fixed::gt_or]).
    ///
    /// ```
    /// # #[cfg(feature = "thermal")] {
    /// use exact_units::{Fixed, Point, origins::Celsius, prefix::One, units::Kelvin};
    ///
    /// type DegC = Point<Celsius, Fixed<Kelvin, One, f32>>;
    /// let limit = DegC::bounded(80.0);
    /// let heat = Fixed::<Kelvin, One, f32>::bounded(f32::INFINITY);
    /// let temp = DegC::bounded(f32::NEG_INFINITY) + heat; // a MaybeNan point
    /// assert!(temp.gt_or(limit, true)); // NaN trips the shutdown
    /// assert!(!(DegC::bounded(25.0) + heat - heat).gt_or(limit, false));
    /// # }
    /// ```
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn gt_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Point<O, Fixed<U, P, T>>: PartialOrd<R>,
    {
        let v = Point::<O, _>::new(self.coord.unchecked_for_compare());
        if nan { !(v <= rhs) } else { v > rhs }
    }

    /// `self >= rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn ge_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Point<O, Fixed<U, P, T>>: PartialOrd<R>,
    {
        let v = Point::<O, _>::new(self.coord.unchecked_for_compare());
        if nan { !(v < rhs) } else { v >= rhs }
    }

    /// `self < rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn lt_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Point<O, Fixed<U, P, T>>: PartialOrd<R>,
    {
        let v = Point::<O, _>::new(self.coord.unchecked_for_compare());
        if nan { !(v >= rhs) } else { v < rhs }
    }

    /// `self <= rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated compare is true for NaN, the `nan` of `true`"
    )]
    pub fn le_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Point<O, Fixed<U, P, T>>: PartialOrd<R>,
    {
        let v = Point::<O, _>::new(self.coord.unchecked_for_compare());
        if nan { !(v > rhs) } else { v <= rhs }
    }

    /// `self == rhs`, and `nan` where `self` is NaN. See [gt_or](Self::gt_or).
    #[inline(always)]
    pub fn eq_or<R: Copy>(self, rhs: R, nan: bool) -> bool
    where
        Point<O, Fixed<U, P, T>>: PartialEq<R>,
    {
        let v = Point::<O, _>::new(self.coord.unchecked_for_compare());
        if nan {
            v == rhs || self.raw().is_nan()
        } else {
            v == rhs
        }
    }
}

impl<O: Origin<Unit = U>, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>>
    Point<O, Fixed<U, P, T, B>>
{
    /// The same point against the origin `O2` in steps `P2` stored as `T2`, if that is exact: `None` when it is not a
    /// whole number of `P2` for an integer `T2`, or doesn't fit `T2`. A modulo `O2` reduces it into its period.
    ///
    /// `O2` has the same unit and [Reference]: a conversion between gauge and absolute pressure doesn't compile
    /// (use [against](Self::against)). The factor and offset are worked out at compile time, so it costs a multiply,
    /// an add and a divide by constants (only the add between origins of the same steps: K to °C); a factor or offset
    /// whose product doesn't fit 128 bits doesn't compile.
    #[inline(always)]
    pub fn to<O2, P2, T2>(self) -> Option<Point<O2, Fixed<U, P2, T2>>>
    where
        O2: Origin<Unit = U, Reference = O::Reference>,
        P2: FixedScale,
        T2: Storage,
    {
        let map = const { Affine::of::<T, T2>(P::SCALE, O::OFFSET, P2::SCALE, O2::OFFSET) };
        let y = match <O2::Modulus as Modulus>::MODULUS {
            None => map.apply::<T, T2>(self.raw())?,
            Some(_) => reduce::<O2::Modulus, P2, T2>(map.apply_repr(self.raw())?)?,
        };
        // a MaybeNan point may be NaN, and NaN is no point of a NaN-free one
        Some(Point::new(Fixed::try_new(y).ok()?))
    }

    /// This point measured against a zero only known at run time: `zero` is where `O`'s [Reference] lies in `O2`.
    /// Gauge pressure with the ambient pressure gives absolute pressure. `None` on overflow, when `O`'s offset isn't a
    /// whole number of `P`, or outside `B2`.
    ///
    /// ```
    /// use exact_units::{Fixed, Point, origins::{Gauge, Vacuum}, prefix::Hecto, units::Pascal};
    ///
    /// let tyre = Point::<Gauge, Fixed<Pascal, Hecto, i32>>::from_raw(2200);
    /// let ambient = Point::<Vacuum, Fixed<Pascal, Hecto, i32>>::from_raw(1013);
    /// assert_eq!(tyre.against(ambient), Some(Point::from_raw(3213)));
    /// ```
    pub fn against<O2, B2>(
        self,
        zero: Point<O2, Fixed<U, P, T, B2>>,
    ) -> Option<Point<O2, Fixed<U, P, T, B2>>>
    where
        O2: Origin<Unit = U>,
        O2::Modulus: PointArith<P, T>,
        B2: Bounds<T>,
    {
        let off = const { Affine::of::<T, T>(Magnitude::ONE, O::OFFSET, P::SCALE, Offset::ZERO) };
        let v = off.apply::<T, T>(T::ZERO)?;
        let v = self.raw().checked_add(v)?;
        let y = <O2::Modulus as PointArith<P, T>>::checked_add(zero.raw(), v)?;
        Fixed::try_new(y).ok().map(Point::new)
    }
}

/// `y` of a conversion reduced into the period of `M` in steps `P2`, into the range of `T2`.
fn reduce<M: Modulus, P2: FixedScale, T2: Storage>(y: Repr) -> Option<T2> {
    match y {
        Repr::Int(x) => {
            let m = const {
                match M::MODULUS {
                    Some(s) => match s.div(P2::SCALE).to_ratio() {
                        #[expect(
                            clippy::as_conversions,
                            clippy::cast_possible_wrap,
                            reason = "n is checked to be at most i128::MAX"
                        )]
                        Some((n, 1)) if n <= i128::MAX as u128 => n as i128,
                        _ => panic!(
                            "the period of a Modulo origin must be a whole number of the point's steps"
                        ),
                    },
                    None => 0,
                }
            };
            T2::from_int(canonical::<T2, i128>(x, m))
        }
        Repr::Float(x) => {
            let m = const {
                match M::MODULUS {
                    Some(s) => s.div(P2::SCALE).to_f64(),
                    None => 0.0,
                }
            };
            T2::from_float(wrap_float(x, m))
        }
    }
}

impl<O, U, P, T, B> Point<O, Fixed<U, P, T, B>>
where
    O: Origin<Unit = U>,
    O::Modulus: PointArith<P, T>,
    T: Storage,
    B: Bounds<T>,
{
    /// `self + v`, `None` on overflow or outside `B`. Never `None` for a modulo origin's integers (they wrap).
    #[inline(always)]
    pub fn checked_add(self, v: Fixed<U, P, T>) -> Option<Self> {
        let y = <O::Modulus as PointArith<P, T>>::checked_add(self.raw(), v.get())?;
        Fixed::try_new(y).ok().map(Self::new)
    }

    /// `self - v`, `None` on overflow or outside `B`.
    #[inline(always)]
    pub fn checked_sub(self, v: Fixed<U, P, T>) -> Option<Self> {
        let y = <O::Modulus as PointArith<P, T>>::checked_sub(self.raw(), v.get())?;
        Fixed::try_new(y).ok().map(Self::new)
    }

    /// The vector from `rhs` to `self`, `None` on overflow (a [Linear] origin's unsigned storage below `rhs`).
    #[inline(always)]
    pub fn checked_diff(self, rhs: Self) -> Option<PointDiff<O, P, T>> {
        <O::Modulus as PointArith<P, T>>::checked_diff(self.raw(), rhs.raw())
            .and_then(Fixed::from_number)
    }
}

/// point + vector, point − vector, point − point: like `T`'s operators (panic on overflow in debug builds; a modulo
/// origin wraps instead). A result outside `B` panics; `checked_add` and friends return `None`. A float point moves
/// by IEEE 754 into a [MaybeNan](crate::MaybeNan) point, as `Fixed`'s `+` does: no check, no panic.
mod arith {
    use super::{Bounds, Fixed, MaybeNan, Origin, Point, PointArith, Storage, Unbounded};
    use core::ops::{Add, AddAssign, Sub, SubAssign};

    impl<O, U, P, T, B> Add<Fixed<U, P, T>> for Point<O, Fixed<U, P, T, B>>
    where
        O: Origin<Unit = U>,
        O::Modulus: PointArith<P, T>,
        T: Storage<Arith = Unbounded>,
        B: Bounds<T>,
    {
        type Output = Self;

        #[inline(always)]
        fn add(self, v: Fixed<U, P, T>) -> Self {
            in_bounds(<O::Modulus as PointArith<P, T>>::add(self.raw(), v.get()))
        }
    }

    impl<O, U, P, T, B> Sub<Fixed<U, P, T>> for Point<O, Fixed<U, P, T, B>>
    where
        O: Origin<Unit = U>,
        O::Modulus: PointArith<P, T>,
        T: Storage<Arith = Unbounded>,
        B: Bounds<T>,
    {
        type Output = Self;

        #[inline(always)]
        fn sub(self, v: Fixed<U, P, T>) -> Self {
            in_bounds(<O::Modulus as PointArith<P, T>>::sub(self.raw(), v.get()))
        }
    }

    impl<O, U, P, T, B> Sub for Point<O, Fixed<U, P, T, B>>
    where
        O: Origin<Unit = U>,
        O::Modulus: PointArith<P, T>,
        T: Storage,
        B: Bounds<T>,
    {
        /// The difference of two float points can be NaN (∞ − ∞): a [MaybeNan](crate::MaybeNan), as for `Fixed`.
        type Output = Fixed<
            U,
            P,
            <O::Modulus as PointArith<P, T>>::Diff,
            <<O::Modulus as PointArith<P, T>>::Diff as Storage>::Arith,
        >;

        #[inline(always)]
        fn sub(self, rhs: Self) -> Self::Output {
            Fixed::new_unchecked(<O::Modulus as PointArith<P, T>>::diff(
                self.raw(),
                rhs.raw(),
            ))
        }
    }

    impl<O, U, P, T, B> AddAssign<Fixed<U, P, T>> for Point<O, Fixed<U, P, T, B>>
    where
        O: Origin<Unit = U>,
        O::Modulus: PointArith<P, T>,
        T: Storage<Arith = Unbounded>,
        B: Bounds<T>,
    {
        #[inline(always)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Point's `+`, whose modulus and bounds say what overflow does"
        )]
        fn add_assign(&mut self, v: Fixed<U, P, T>) {
            *self = *self + v;
        }
    }

    impl<O, U, P, T, B> SubAssign<Fixed<U, P, T>> for Point<O, Fixed<U, P, T, B>>
    where
        O: Origin<Unit = U>,
        O::Modulus: PointArith<P, T>,
        T: Storage<Arith = Unbounded>,
        B: Bounds<T>,
    {
        #[inline(always)]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "the same op as Point's `-`, whose modulus and bounds say what overflow does"
        )]
        fn sub_assign(&mut self, v: Fixed<U, P, T>) {
            *self = *self - v;
        }
    }

    /// point ± vector of a float, either one [MaybeNan] or not: the float's own operation, a [MaybeNan] point
    /// (∞ − ∞ is NaN), as for `Fixed`. `+=` and `-=` on a [MaybeNan] point.
    macro_rules! float {
        ($($ty:ident: $($b:ident $bv:ident),*;)*) => {$($(
            impl<O, U, P> Add<Fixed<U, P, $ty, $bv>> for Point<O, Fixed<U, P, $ty, $b>>
            where
                O: Origin<Unit = U>,
                O::Modulus: PointArith<P, $ty>,
            {
                type Output = Point<O, Fixed<U, P, $ty, MaybeNan>>;

                #[inline(always)]
                fn add(self, v: Fixed<U, P, $ty, $bv>) -> Self::Output {
                    Point::new(Fixed::new_unchecked(<O::Modulus as PointArith<P, $ty>>::add(self.raw(), v.get())))
                }
            }

            impl<O, U, P> Sub<Fixed<U, P, $ty, $bv>> for Point<O, Fixed<U, P, $ty, $b>>
            where
                O: Origin<Unit = U>,
                O::Modulus: PointArith<P, $ty>,
            {
                type Output = Point<O, Fixed<U, P, $ty, MaybeNan>>;

                #[inline(always)]
                fn sub(self, v: Fixed<U, P, $ty, $bv>) -> Self::Output {
                    Point::new(Fixed::new_unchecked(<O::Modulus as PointArith<P, $ty>>::sub(self.raw(), v.get())))
                }
            }
        )*

            impl<O, U, P, BV: Bounds<$ty>> AddAssign<Fixed<U, P, $ty, BV>> for Point<O, Fixed<U, P, $ty, MaybeNan>>
            where
                O: Origin<Unit = U>,
                O::Modulus: PointArith<P, $ty>,
            {
                #[inline(always)]
                fn add_assign(&mut self, v: Fixed<U, P, $ty, BV>) {
                    *self = Point::new(Fixed::new_unchecked(<O::Modulus as PointArith<P, $ty>>::add(self.raw(), v.get())));
                }
            }

            impl<O, U, P, BV: Bounds<$ty>> SubAssign<Fixed<U, P, $ty, BV>> for Point<O, Fixed<U, P, $ty, MaybeNan>>
            where
                O: Origin<Unit = U>,
                O::Modulus: PointArith<P, $ty>,
            {
                #[inline(always)]
                fn sub_assign(&mut self, v: Fixed<U, P, $ty, BV>) {
                    *self = Point::new(Fixed::new_unchecked(<O::Modulus as PointArith<P, $ty>>::sub(self.raw(), v.get())));
                }
            }
        )*};
    }

    float! {
        f32: Unbounded Unbounded, Unbounded MaybeNan, MaybeNan Unbounded, MaybeNan MaybeNan;
        f64: Unbounded Unbounded, Unbounded MaybeNan, MaybeNan Unbounded, MaybeNan MaybeNan;
    }

    #[inline(always)]
    #[expect(
        clippy::panic,
        reason = "`+` and `-` of a bounded point panic outside its bounds, as documented; checked_add is the \
                  non-panicking form"
    )]
    fn in_bounds<O, U, P, T: Storage, B: Bounds<T>>(y: T) -> Point<O, Fixed<U, P, T, B>> {
        let Ok(f) = Fixed::try_new(y) else {
            panic!("a point moved outside its bounds")
        };
        Point::new(f)
    }
}

/// `y = (x · num + add) / den`, the factor and offset between two origins and steps, worked out at compile time.
#[derive(Copy, Clone)]
struct Affine {
    num: i128,
    add: i128,
    den: i128,
    /// `num`, `add` and `den` in 64 bits, for the narrow path only (where they fit, checked in [of](Affine::of)).
    num64: i64,
    add64: i64,
    den64: i64,
    /// In 64 bits: both storages of up to 32 bits and `|x · num| + |add|` fits `i64`.
    narrow: bool,
    float: bool,
    num_f: f64,
    add_f: f64,
    den_f: f64,
}

impl Affine {
    /// From `x` steps of `step1` above `off1` to steps of `step2` above `off2`: `y · step2 + off2 = x · step1 + off1`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a conversion between points that does not fit its widened arithmetic"
    )]
    const fn of<T: Storage, T2: Storage>(
        step1: Magnitude,
        off1: Offset,
        step2: Magnitude,
        off2: Offset,
    ) -> Self {
        let float = T::IS_FLOAT || T2::IS_FLOAT;
        let factor = step1.div(step2);
        let shift = off1.sub(off2).mul(step2.recip());
        let (an, ad) = match factor.to_ratio() {
            Some(r) => r,
            None if float => (1, 1),
            None => panic!(
                "an integer Point converts by a fraction of integers only: this factor has a power of π, ln 2 or \
                 ln 10 or a half exponent (convert through a float storage)"
            ),
        };
        let Some((bn, bd)) = shift.to_ratio() else {
            panic!("the offset between these origins is not a fraction of integers")
        };
        let (Some(num), Some(add), Some(den)) = (mul_i(an, bd), mul_s(bn, ad), mul_i(ad, bd))
        else {
            panic!("the factor and offset between these origins don't fit 128 bits")
        };
        let g = offset::gcd(
            offset::gcd(num.unsigned_abs(), add.unsigned_abs()),
            den.unsigned_abs(),
        );
        #[expect(
            clippy::cast_possible_wrap,
            reason = "g divides den, which is in 1..=i128::MAX"
        )]
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "g ≥ 1 divides den, which is in 1..=i128::MAX: no division by 0, and g as i128 is positive"
        )]
        let (num, add, den) = (num / g as i128, add / g as i128, den / g as i128);
        let max_abs = if T::MAX_INT.unsigned_abs() > T::MIN_INT.unsigned_abs() {
            T::MAX_INT.unsigned_abs()
        } else {
            T::MIN_INT.unsigned_abs()
        };
        let mut narrow = false;
        if !float {
            let Some(top) = max_abs.checked_mul(num.unsigned_abs()) else {
                panic!(
                    "the factor between these origins doesn't fit the widened multiply (128 bits)"
                )
            };
            let Some(top) = top.checked_add(add.unsigned_abs()) else {
                panic!(
                    "the factor between these origins doesn't fit the widened multiply (128 bits)"
                )
            };
            assert!(
                top <= i128::MAX as u128,
                "the factor between these origins doesn't fit the widened multiply (128 bits)"
            );
            // a den past i64 divides only in 128 bits
            narrow = T::BITS <= 32
                && T2::BITS <= 32
                && top <= i64::MAX as u128
                && den <= i64::MAX as i128;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "the float forms are approximations by design, f64 holds 53 bits"
        )]
        let (num_f, den_f) = match factor.to_ratio() {
            Some(_) => (num as f64, den as f64),
            None => (factor.to_f64(), 1.0),
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "used on the narrow path only, where |num|, |add| and den fit i64 (checked above)"
        )]
        let (num64, add64, den64) = (num as i64, add as i64, den as i64);
        Affine {
            num,
            add,
            den,
            num64,
            add64,
            den64,
            narrow,
            float,
            num_f,
            #[expect(
                clippy::cast_precision_loss,
                reason = "the float form is an approximation by design, f64 holds 53 bits"
            )]
            add_f: if factor.is_rational() {
                add as f64
            } else {
                shift.to_f64()
            },
            den_f,
        }
    }

    /// `y` before it is fitted into a storage: what a modulo origin reduces first.
    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "|x · num| + |add| ≤ i128::MAX for every x of T, checked in of(); den ≥ 1"
    )]
    fn apply_repr<T: Storage>(self, x: T) -> Option<Repr> {
        let x = x.to_repr();
        if self.float {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "the float path rounds by design, f64 holds 53 bits (no From)"
            )]
            let x = match x {
                Repr::Int(i) => i as f64,
                Repr::Float(f) => f,
            };
            return Some(Repr::Float((x * self.num_f + self.add_f) / self.den_f));
        }
        let Repr::Int(x) = x else {
            return None;
        };
        let y = x * self.num + self.add;
        if self.den != 1 && y % self.den != 0 {
            return None;
        }
        Some(Repr::Int(y / self.den))
    }

    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "|x · num| + |add| fits i128 for every x of T, and i64 on the narrow path, both checked in of(); \
                  den ≥ 1"
    )]
    fn apply<T: Storage, T2: Storage>(self, x: T) -> Option<T2> {
        if self.float {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "the float path rounds by design, f64 holds 53 bits (no From)"
            )]
            let x = match x.to_repr() {
                Repr::Int(i) => i as f64,
                Repr::Float(f) => f,
            };
            return T2::from_float((x * self.num_f + self.add_f) / self.den_f);
        }
        if self.narrow {
            let mut y = x.to_i64();
            if self.num != 1 {
                y *= self.num64;
            }
            if self.add != 0 {
                y += self.add64;
            }
            if self.den != 1 {
                let den = self.den64;
                if y % den != 0 {
                    return None;
                }
                y /= den;
            }
            T2::from_i64(y)
        } else {
            let Repr::Int(x) = x.to_repr() else {
                return None;
            };
            let mut y = x * self.num + self.add;
            if self.den != 1 {
                if y % self.den != 0 {
                    return None;
                }
                y /= self.den;
            }
            T2::from_int(y)
        }
    }
}

#[expect(
    clippy::cast_possible_wrap,
    reason = "p is checked to be at most i128::MAX"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn mul_i(a: u128, b: u128) -> Option<i128> {
    match a.checked_mul(b) {
        Some(p) if p <= i128::MAX as u128 => Some(p as i128),
        _ => None,
    }
}

#[expect(
    clippy::cast_possible_wrap,
    reason = "b is checked to be at most i128::MAX"
)]
#[expect(
    clippy::as_conversions,
    reason = "const fn: From is not const on stable"
)]
const fn mul_s(a: i128, b: u128) -> Option<i128> {
    if b > i128::MAX as u128 {
        return None;
    }
    a.checked_mul(b as i128)
}

/// `n / d` without a common factor, to write as a decimal: exactly when `d` has no prime but 2 and 5.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "g ≥ 1 (the max), and an unsigned division by it cannot overflow"
)]
pub(crate) fn ratio(n: i128, d: i128) -> Number {
    let g = offset::gcd(n.unsigned_abs(), d.unsigned_abs()).max(1);
    Number::ratio(
        (n < 0) != (d < 0),
        n.unsigned_abs() / g,
        d.unsigned_abs() / g,
    )
}

/// `n / d` as [ratio] writes it, with its sign.
pub(crate) fn write_ratio(n: i128, d: i128, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let x = ratio(n, d);
    if x.is_negative() {
        f.write_str("-")?;
    }
    x.write_abs(None, f)
}

/// The number on the origin's own scale with its symbol: `25.5 °C`, `98 °F`, exact where the steps are decimal. A
/// precision rounds the number as [Fixed]'s `{:#}` does (`{:.1}`); width, fill and alignment take the symbol in.
impl<O: Origin<Unit = U>, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> fmt::Display
    for Point<O, Fixed<U, P, T, B>>
{
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "the float is the approximation shown where the ratio doesn't fit (no From)"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let map = const { Affine::of::<T, f64>(P::SCALE, O::OFFSET, O::STEP, O::SHOWN) };
        let x = match self.raw().to_repr() {
            Repr::Int(x) if P::SCALE.div(O::STEP).is_rational() => {
                match x.checked_mul(map.num).and_then(|y| y.checked_add(map.add)) {
                    Some(y) => ratio(y, map.den),
                    None => Number::Float((x as f64 * map.num_f + map.add_f) / map.den_f),
                }
            }
            Repr::Int(x) => Number::Float((x as f64 * map.num_f + map.add_f) / map.den_f),
            Repr::Float(x) => Number::Float((x * map.num_f + map.add_f) / map.den_f),
        };
        let prec = f.precision();
        pad(f, x.is_negative(), |f| {
            x.write_abs(prec, f)?;
            if !O::SYMBOL.is_empty() {
                write!(f, " {}", O::SYMBOL)?;
            }
            Ok(())
        })
    }
}

/// `Celsius(25 °C, 2500 cK i16)`: the origin, the point as shown, then its number as the [Fixed] shows it.
impl<O: Origin<Unit = U>, U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> fmt::Debug
    for Point<O, Fixed<U, P, T, B>>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({self}, {} {})", O::NAME, self.coord, T::NAME)
    }
}

/// `Celsius 2500 cK`: the origin's name and the number as the [Fixed] formats it.
#[cfg(feature = "defmt")]
impl<O: Origin, F: defmt::Format> defmt::Format for Point<O, F> {
    fn format(&self, f: defmt::Formatter<'_>) {
        defmt::write!(f, "{=str} {}", O::NAME, self.coord);
    }
}

impl<O, F: Clone> Clone for Point<O, F> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Point::new(self.coord.clone())
    }
}

impl<O, F: Copy> Copy for Point<O, F> {}

impl<O, F: Default> Default for Point<O, F> {
    fn default() -> Self {
        Point::new(F::default())
    }
}

impl<O, F: PartialEq> PartialEq for Point<O, F> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.coord == other.coord
    }
}

impl<O, F: Eq> Eq for Point<O, F> {}

impl<O, F: PartialOrd> PartialOrd for Point<O, F> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.coord.partial_cmp(&other.coord)
    }
}

impl<O, F: Ord> Ord for Point<O, F> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.coord.cmp(&other.coord)
    }
}

impl<O, F: Hash> Hash for Point<O, F> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.coord.hash(state);
    }
}
