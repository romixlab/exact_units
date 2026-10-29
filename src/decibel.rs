//! Levels in decibels of a gain, the form (10·lg or 20·lg) from the unit's [Kind](crate::Kind): a ratio of root-power
//! quantities (V/V, A/A, Pa/Pa) takes 20·lg, one of powers (W/W) 10·lg, and any other kind, a plain
//! [Ratio](crate::units::Ratio) too, has no level and doesn't compile.
//!
//! - [Fixed::to_db] / [Fixed::from_db] in `f64`, [Fixed::to_db_f32] / [Fixed::from_db_f32] in `f32`, the float a
//!   Cortex-M4F or M33 has in hardware (`f64` is software there). With feature `std` or `libm`: `log10` and `powf`
//!   aren't in `core`. With `libm` (pure Rust, no C) they come from it even when `std` is on, so a host and a
//!   firmware compute the same bits.
//! - [Fixed::from_db_const] and [db!](crate::db): a level written in decimal (`db!(-3.01)`) becomes the stored value
//!   at compile time, exactly: the stored value nearest to `10^(dB/20)` (or `/10`), worked out in integers with an
//!   error bound, no float math at run time and no feature needed. A firmware compares the integers.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, db, prefix::Milli, units::{Per, Volt}};
//! type Gain = Fixed<Per<Volt, Volt>, Milli, u16>;
//! const HALF_POWER: Gain = db!(-3.0103);
//! assert_eq!(HALF_POWER.get(), 707); // 0.707 V/V
//! const PLUS_20: Gain = Gain::from_db_const(exact_units::decibel::Decibels::new(20, 0));
//! assert_eq!(PLUS_20.get(), 10_000);
//! # }
//! ```

mod exact;

use crate::{Bounds, Fixed, FixedScale, LevelForm, Unit};

/// A level in dB as a decimal, `num · 10^-decimals`: what [db!](crate::db) parses its literal into.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Decibels {
    num: i64,
    decimals: u8,
}

/// Decimals a [Decibels] takes: 10⁻⁹ dB is far below any level a device can tell apart.
pub const MAX_DECIMALS: u8 = 9;

impl Decibels {
    /// `num · 10^-decimals` dB: `Decibels::new(-301, 2)` is −3.01 dB. Up to [MAX_DECIMALS] decimals.
    pub const fn new(num: i64, decimals: u8) -> Decibels {
        assert!(
            decimals <= MAX_DECIMALS,
            "a level in dB takes up to 9 decimals"
        );
        Decibels { num, decimals }
    }

    /// A decimal number as written: `"-3.01"`, `"6"`, `"+0.5"`. Panics (a compile error in `const` code) on
    /// anything else.
    pub const fn parse(s: &str) -> Decibels {
        let b = s.as_bytes();
        let mut i = 0;
        let mut neg = false;
        if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
            neg = b[i] == b'-';
            i += 1;
        }
        // stringify! of a negative literal may keep a space after the sign
        while i < b.len() && b[i] == b' ' {
            i += 1;
        }
        let mut num: i64 = 0;
        let mut decimals = 0u8;
        let mut digits = 0;
        let mut point = false;
        while i < b.len() {
            match b[i] {
                b'0'..=b'9' => {
                    num = match num.checked_mul(10) {
                        Some(n) => match n.checked_add((b[i] - b'0') as i64) {
                            Some(n) => n,
                            None => panic!("a level in dB too long to parse"),
                        },
                        None => panic!("a level in dB too long to parse"),
                    };
                    digits += 1;
                    if point {
                        decimals += 1;
                    }
                }
                b'.' if !point => point = true,
                b'_' => {}
                _ => panic!("a level in dB is a decimal number: -3, 6.02, +0.5"),
            }
            i += 1;
        }
        assert!(
            digits > 0,
            "a level in dB is a decimal number: -3, 6.02, +0.5"
        );
        Decibels::new(if neg { -num } else { num }, decimals)
    }

    /// The digits, `-301` for −3.01 dB.
    pub const fn num(self) -> i64 {
        self.num
    }

    /// The decimals, `2` for −3.01 dB.
    pub const fn decimals(self) -> u8 {
        self.decimals
    }

    /// The level as an `f64` (rounded).
    pub const fn to_f64(self) -> f64 {
        self.num as f64 / crate::num::pow10_f64(self.decimals as i8)
    }

    /// `n / d` with `10^(n/d)` the gain of form `form`, reduced.
    const fn exponent(self, form: LevelForm) -> (i64, u64) {
        let per_decade = match form {
            LevelForm::Power => 10,
            LevelForm::RootPower => 20,
        };
        let n = self.num;
        let d = per_decade * 10u64.pow(self.decimals as u32);
        let g = gcd(n.unsigned_abs(), d);
        (n / g as i64, d / g)
    }
}

const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// A gain at a level in dB, worked out at compile time: `db!(-3.01)` is the [Fixed] its context names, holding the
/// stored value nearest to the exact gain. It is an associated const ([AtLevel]), so nothing of it runs at run time;
/// [Fixed::from_db_const] is the same as a `const fn`, for a level that isn't a literal.
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Fixed, db, prefix::{Micro, One}, units::{Per, Volt, Watt}};
/// let g: Fixed<Per<Volt, Volt>, Micro, u32> = db!(6);
/// assert_eq!(g.get(), 1_995_262); // 10^(6/20) = 1.9952623..
/// let p: Fixed<Per<Watt, Watt>, One, f32> = db!(-3);
/// assert_eq!(p.get(), 0.5011872);
/// # }
/// ```
///
/// A kind without a level form doesn't compile, nor does a level whose gain doesn't fit the storage:
///
/// ```compile_fail
/// use exact_units::{Fixed, db, prefix::One, units::Ratio};
/// let _: Fixed<Ratio, One, u16> = db!(-3);
/// ```
///
/// ```compile_fail
/// # #[cfg(not(feature = "electrical"))] compile_error!("needs electrical");
/// use exact_units::{Fixed, db, prefix::Milli, units::{Per, Volt}};
/// let _: Fixed<Per<Volt, Volt>, Milli, u16> = db!(40); // 100 V/V is 100_000 mV/V
/// ```
#[macro_export]
macro_rules! db {
    ($level:literal) => {
        <_ as $crate::decibel::AtLevel<
            $crate::decibel::Level<
                { $crate::decibel::Decibels::parse(stringify!($level)).num() },
                { $crate::decibel::Decibels::parse(stringify!($level)).decimals() },
            >,
        >>::AT
    };
}

/// A level in dB as a type, `NUM · 10^-DECIMALS` dB: what [db!](crate::db) names, so the storage of the [Fixed] can
/// come from the context.
pub struct Level<const NUM: i64, const DECIMALS: u8>;

/// The gain at level `L` as an associated const, worked out at compile time ([Fixed::from_db_const]): what
/// [db!](crate::db) reads, for every [Fixed] with an integer or float storage.
pub trait AtLevel<L>: Sized {
    const AT: Self;
}

/// The exact gain of `db` in units of `P · U` as an interval, or a compile error for a scale that isn't a fraction of
/// integers.
const fn gain<U: Unit, P: FixedScale>(db: Decibels, form: LevelForm) -> exact::Interval {
    let unit = match U::EXPR.magnitude() {
        Some(m) => m,
        None => panic!("a level needs an exact scale"),
    };
    let (num, den) = match P::SCALE.mul(unit).to_ratio() {
        Some(r) => r,
        None => panic!("a level at compile time needs a scale that is a fraction of integers"),
    };
    let (n, d) = db.exponent(form);
    // the value in units of the scale: 10^(n/d) / (num / den)
    exact::Interval::level(n, d, num, den)
}

/// `from_db_const` per storage: the integers through `bounded`, the floats checked against `B` here.
macro_rules! from_db_const {
    (int $($ty:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The gain at `db`, rounded to the nearest stored value (ties to even) at compile time: see the
            /// [module](crate::decibel). A compile error in `const` code (a panic at run time) for a kind without a
            /// level form, a gain that doesn't fit `T` or `B`, or one too close to halfway between two stored values
            /// to decide.
            pub const fn from_db_const(db: Decibels) -> Self {
                let x = gain::<U, P>(db, Self::LEVEL_FORM).to_int();
                if x > $ty::MAX as u128 {
                    panic!("this level is too large for its storage");
                }
                Self::bounded(x as $ty)
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, const N: i64, const D: u8> AtLevel<Level<N, D>>
            for Fixed<U, P, $ty, B>
        {
            const AT: Self = Self::from_db_const(Decibels::new(N, D));
        }
    )*};
    (float $($ty:ident $to:ident)*) => {$(
        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The gain at `db`, the nearest float (ties to even) at compile time: see the
            /// [module](crate::decibel). A compile error in `const` code (a panic at run time) for a kind without a
            /// level form or a gain outside `B` or the float's normal range.
            pub const fn from_db_const(db: Decibels) -> Self {
                let x = gain::<U, P>(db, Self::LEVEL_FORM).$to();
                if let Some(min) = B::MIN
                    && x < min
                {
                    panic!("value below its bound");
                }
                if let Some(max) = B::MAX
                    && x > max
                {
                    panic!("value above its bound");
                }
                Self::new_unchecked(x)
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, const N: i64, const D: u8> AtLevel<Level<N, D>>
            for Fixed<U, P, $ty, B>
        {
            const AT: Self = Self::from_db_const(Decibels::new(N, D));
        }
    )*};
}

from_db_const!(int u8 u16 u32 u64 i8 i16 i32 i64);
from_db_const!(float f32 to_f32 f64 to_f64);

/// `log10` and `10^x`: from libm when it is on, so every target computes the same bits, else from std.
#[cfg(any(feature = "std", feature = "libm"))]
mod math {
    #[cfg(feature = "libm")]
    pub(super) use libm::{exp10, exp10f, log10, log10f};

    #[cfg(not(feature = "libm"))]
    #[inline(always)]
    pub(super) fn log10(x: f64) -> f64 {
        x.log10()
    }
    #[cfg(not(feature = "libm"))]
    #[inline(always)]
    pub(super) fn log10f(x: f32) -> f32 {
        x.log10()
    }
    #[cfg(not(feature = "libm"))]
    #[inline(always)]
    pub(super) fn exp10(x: f64) -> f64 {
        10f64.powf(x)
    }
    #[cfg(not(feature = "libm"))]
    #[inline(always)]
    pub(super) fn exp10f(x: f32) -> f32 {
        10f32.powf(x)
    }
}

impl<U: Unit, P: FixedScale, T: crate::Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The form a level of `U` takes, or a compile error for a kind without one.
    const LEVEL_FORM: LevelForm = match U::KIND.level_form() {
        Some(form) => form,
        None => panic!(
            "decibels are for a ratio of powers (W/W) or of root-power quantities (V/V, A/A, Pa/Pa): a plain \
             Ratio or another kind has no level form"
        ),
    };
}

#[cfg(any(feature = "std", feature = "libm"))]
impl<U: Unit, P: FixedScale, T: crate::Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// The gain in dB: 20·lg for a ratio of root-power quantities (V/V, A/A), 10·lg for one of powers (W/W). A
    /// kind without a level form (a plain [Ratio](crate::units::Ratio), a duty cycle) doesn't compile. Feature
    /// `std` or `libm`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, prefix::{Milli, One}, units::{Ampere, Per, Volt, Watt}};
    /// assert_eq!(Fixed::<Per<Volt, Volt>, One, f64>::new(10.0).to_db(), 20.0);
    /// assert_eq!(Fixed::<Per<Ampere, Ampere>, Milli, u32>::new(100).to_db(), -20.0);
    /// assert_eq!(Fixed::<Per<Watt, Watt>, One, f64>::new(100.0).to_db(), 20.0);
    /// # }
    /// ```
    ///
    /// ```compile_fail
    /// use exact_units::{Fixed, prefix::One, units::Ratio};
    /// let _ = Fixed::<Ratio, One, f64>::new(0.5).to_db();
    /// ```
    pub fn to_db(self) -> f64 {
        let form = const { Self::LEVEL_FORM };
        let x = match self.get().to_repr() {
            crate::Repr::Int(i) => i as f64,
            crate::Repr::Float(x) => x,
        };
        let x = x * const { P::SCALE.to_f64() * U::EXPR.factor() };
        form.db_per_decade() * math::log10(x)
    }

    /// The gain of a level in dB, by the form of [to_db](Self::to_db): `None` where it isn't a number of `T` (an
    /// integer storage takes whole numbers only) or not within `B`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Volt, Watt}};
    /// assert_eq!(Fixed::<Per<Volt, Volt>, Milli, u32>::from_db(40.0), Some(Fixed::new(100_000)));
    /// assert_eq!(Fixed::<Per<Watt, Watt>, One, f64>::from_db(30.0), Some(Fixed::new(1000.0)));
    /// # }
    /// ```
    pub fn from_db(db: f64) -> Option<Self> {
        let form = const { Self::LEVEL_FORM };
        let x = math::exp10(db / form.db_per_decade());
        let x = x / const { P::SCALE.to_f64() * U::EXPR.factor() };
        Self::try_new(T::from_float(x)?).ok()
    }

    /// [to_db](Self::to_db) in `f32`, with no `f64` on the way: the float of a Cortex-M4F or M33.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Volt}};
    /// assert_eq!(Fixed::<Per<Volt, Volt>, One, f32>::new(10.0).to_db_f32(), 20.0);
    /// assert_eq!(Fixed::<Per<Volt, Volt>, Milli, u16>::new(100).to_db_f32(), -20.0);
    /// # }
    /// ```
    pub fn to_db_f32(self) -> f32 {
        let per_decade = const { Self::LEVEL_FORM.db_per_decade() as f32 };
        let x = self.get().to_f32() * const { (P::SCALE.to_f64() * U::EXPR.factor()) as f32 };
        per_decade * math::log10f(x)
    }

    /// [from_db](Self::from_db) in `f32`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, prefix::{Milli, One}, units::{Per, Watt}};
    /// assert_eq!(Fixed::<Per<Watt, Watt>, Milli, u32>::from_db_f32(20.0), Some(Fixed::new(100_000)));
    /// # }
    /// ```
    pub fn from_db_f32(db: f32) -> Option<Self> {
        let per_decade = const { Self::LEVEL_FORM.db_per_decade() as f32 };
        // times the reciprocal: 1000.0 for Milli is exact in f32, 0.001 isn't
        let x = math::exp10f(db / per_decade)
            * const { (1.0 / (P::SCALE.to_f64() * U::EXPR.factor())) as f32 };
        Self::try_new(T::from_f32(x)?).ok()
    }
}
