//! Decimal literals as stored values, worked out at compile time: `fixed!(0.1)` is the [Fixed] its context names,
//! holding the stored value nearest to 0.1 of its unit, so a constant in Q30, in steps of 2⁻¹⁵ or in mV is written as
//! the number it means, never as `107_374_182 // 0.1 · 2³⁰`.
//!
//! - Any scale that is a fraction of integers: `Pow2`, `Pow10`, `Rational`, the SI prefixes and their products. A
//!   scale with π or a root in it is a compile error: its stored values aren't decimals.
//! - Integer storages take the nearest integer, ties to even, from the exact fraction: nothing is rounded on the way.
//!   Float storages take the nearest float, ties to even.
//! - A value that doesn't fit the storage or is outside the bounds `B` is a compile error, as is a negative value of
//!   an unsigned storage.
//! - The literal is in units of `U`: `fixed!(0.25)` of a `Fixed<Volt, Milli, u16>` is 0.25 V, stored as 250.
//!   A decimal exponent works too: `fixed!(2.5e-3)`.
//! - `fixed!(0.5, -0.25)` is a [Complex] of two such parts.
//! - `fixed!(17.7 in Decibel)` is a literal in another unit `S` of the same dimension and kind
//!   ([Fixed::from_decimal_in]): a level in dB into a storage of µNp, degrees into radians. A factor with π, ln 10 or
//!   a root in it is rounded from an interval around the exact value, and a literal too close to halfway between two
//!   stored values to decide is a compile error.
//! - `fixed!(100 m/s)` names that other unit `S` the way [q!](crate::q) reads one, through its symbol table or a
//!   type (`fixed!(100 Speed)`), rather than writing a type path by hand: the same rules as `fixed!(17.7 in
//!   Decibel)` once the symbol resolves to a unit.
//!
//! It is an associated const ([AtLiteral]), so nothing of it runs at run time; [Fixed::from_decimal] is the same as
//! a `const fn`, for a value that isn't a literal.
//!
//! ```
//! use exact_units::{Fixed, fixed, prefix::{Milli, Pow2, Rational}, units::Ratio};
//! const BUSY: Fixed<Ratio, Pow2<-30>, i64> = fixed!(0.1);
//! assert_eq!(BUSY.get(), 107_374_182); // 0.1 · 2³⁰ = 107374182.4
//! const HALF: Fixed<Ratio, Pow2<-15>, i16> = fixed!(-0.5);
//! assert_eq!(HALF.get(), -16384);
//! let third: Fixed<Ratio, Rational<1, 3>, u8> = fixed!(2.5); // 7.5 thirds, ties to even
//! assert_eq!(third.get(), 8);
//! let x: Fixed<Ratio, Milli, f32> = fixed!(1.5e-3);
//! assert_eq!(x.get(), 1.5);
//! ```

use crate::complex::Complex;
use crate::decibel::exact::Interval;
use crate::{Bounds, Fixed, FixedScale, Magnitude, Unit, Unlimited};

/// A decimal number, `num · 10^exp`: what [fixed!](crate::fixed) parses its literal into.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Decimal {
    num: i64,
    exp: i16,
}

impl Decimal {
    /// `num · 10^exp`: `Decimal::new(25, -3)` is 0.025.
    #[must_use]
    pub const fn new(num: i64, exp: i16) -> Decimal {
        Decimal { num, exp }
    }

    /// A decimal number as written: `"0.1"`, `"-3"`, `"+2.5e-3"`, `"1_000"`. Panics (a compile error in `const`
    /// code) on anything else, a type suffix (`0.1f32`) included: the storage comes from the context.
    #[must_use]
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::indexing_slicing,
        clippy::panic,
        reason = "every b[i] is read after i < b.len(); a malformed literal is a compile error in const code (a panic at run time), as documented"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "i < b.len(), the loop bounds; b[i] is a digit there, so b[i] - b'0' is 0..=9; num is built non-negative, so -num fits; digits, edigits and exp move by one per byte of s, so they fit i32 for any s below 2^31 bytes: past that they wrap, a known, accepted limit for a literal (not a TODO)"
    )]
    pub const fn parse(s: &str) -> Decimal {
        const FORM: &str = "a literal is a decimal number: 0.1, -3, 2.5e-3";
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
        let mut exp: i32 = 0;
        let mut digits = 0;
        let mut point = false;
        while i < b.len() {
            match b[i] {
                b'0'..=b'9' => {
                    num = match num.checked_mul(10) {
                        Some(n) => match n.checked_add((b[i] - b'0') as i64) {
                            Some(n) => n,
                            None => panic!("a literal of more than 18 digits"),
                        },
                        None => panic!("a literal of more than 18 digits"),
                    };
                    digits += 1;
                    if point {
                        exp -= 1;
                    }
                }
                b'.' if !point => point = true,
                b'_' => {}
                b'e' | b'E' => break,
                _ => panic!("{}", FORM),
            }
            i += 1;
        }
        assert!(digits > 0, "{}", FORM);
        if i < b.len() {
            // the exponent after e
            i += 1;
            let mut eneg = false;
            if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
                eneg = b[i] == b'-';
                i += 1;
            }
            let mut e: i32 = 0;
            let mut edigits = 0;
            while i < b.len() {
                match b[i] {
                    // saturated: an exponent past i32 is out of range for any s below 2^31 bytes, whatever
                    // the digits after the point take off
                    b'0'..=b'9' => e = e.saturating_mul(10).saturating_add((b[i] - b'0') as i32),
                    b'_' => {}
                    _ => panic!("{}", FORM),
                }
                if b[i] != b'_' {
                    edigits += 1;
                }
                i += 1;
            }
            assert!(edigits > 0, "{}", FORM);
            exp = if eneg {
                exp.saturating_sub(e)
            } else {
                exp.saturating_add(e)
            };
        }
        assert!(
            exp >= i16::MIN as i32 && exp <= i16::MAX as i32,
            "a literal's exponent is out of range"
        );
        #[expect(
            clippy::cast_possible_truncation,
            reason = "exp is checked to be in i16's range just above"
        )]
        let exp = exp as i16;
        Decimal::new(if neg { -num } else { num }, exp)
    }

    /// The digits, `25` for 0.025.
    #[must_use]
    pub const fn num(self) -> i64 {
        self.num
    }

    /// The power of ten, `-3` for 0.025.
    #[must_use]
    pub const fn exp(self) -> i16 {
        self.exp
    }

    /// `|self| / scale` as a reduced fraction `(p, q)`, or a compile error for a scale that isn't a fraction of
    /// integers or a fraction too large for `u128`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code (a panic at run time), as documented: a scale that is not a fraction of integers, or a fraction past u128"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "every g is a gcd with one side at least 1 (q starts at 1 or 10^k, p at |num| >= 1: both callers return early for 0), so g >= 1"
    )]
    const fn over(self, scale: Magnitude) -> (u128, u128) {
        let Some((snum, sden)) = scale.to_ratio() else {
            panic!(
                "a literal needs a scale that is a fraction of integers: the stored values of a scale with π or a \
                 root in it aren't decimals"
            )
        };
        let Some(ten) = 10u128.checked_pow(self.exp.unsigned_abs() as u32) else {
            panic!("a literal's exponent is out of range")
        };
        let (mut p, mut q) = (self.num.unsigned_abs() as u128, 1u128);
        if self.exp >= 0 {
            p = mul(p, ten);
        } else {
            q = ten;
        }
        let g = gcd(p, q);
        (p, q) = (p / g, q / g);
        // p · sden / (q · snum), each product reduced against the other side first
        let g = gcd(sden, q);
        let (sden, q) = (sden / g, q / g);
        let g = gcd(p, snum);
        let (p, snum) = (p / g, snum / g);
        (mul(p, sden), mul(q, snum))
    }

    /// The stored value nearest to `self` in units of `P`, ties to even, as an `i128`.
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "q >= 1: a product of factors at least 1 (to_ratio's num is a product of prime powers); rem < q, so q - rem fits; up only when rem > 0, so q >= 2 and int + 1 fits; 0 <= x <= i128::MAX, so -x fits"
    )]
    const fn to_int(self, scale: Magnitude) -> i128 {
        if self.num == 0 {
            return 0;
        }
        let (p, q) = self.over(scale);
        let (int, rem) = (p / q, p % q);
        // rem > q/2, or rem == q/2 and int is odd; q - rem can't overflow where 2 · rem could
        let up = rem > q - rem || (rem == q - rem && int & 1 == 1);
        let x = int + up as u128;
        assert!(
            x <= i128::MAX as u128,
            "this literal is too large for its storage"
        );
        #[expect(
            clippy::cast_possible_wrap,
            reason = "x is checked to be at most i128::MAX just above"
        )]
        let x = x as i128;
        if self.num < 0 { -x } else { x }
    }

    /// The exact `|self|` in units of `P` as an interval, for the nearest float; `None` for zero.
    const fn to_interval(self, scale: Magnitude) -> Option<Interval> {
        if self.num == 0 {
            return None;
        }
        let (p, q) = self.over(scale);
        Some(Interval::ratio(p, q))
    }

    /// As [to_interval](Self::to_interval), also for a scale that isn't a fraction of integers (π, ln 10, a root):
    /// `|self| / scale` with the scale's interval ([Interval::magnitude]).
    const fn to_interval_any(self, scale: Magnitude) -> Option<Interval> {
        if scale.is_rational() {
            return self.to_interval(scale);
        }
        match self.to_interval(Magnitude::ONE) {
            Some(x) => Some(x.mul(Interval::magnitude(scale.recip()))),
            None => None,
        }
    }

    /// As [to_int](Self::to_int), also for a scale that isn't a fraction of integers: then the interval of the exact
    /// value must round to one integer at both ends, else a compile error (a literal too close to halfway between
    /// two stored values to decide).
    #[expect(
        clippy::as_conversions,
        reason = "const fn: From is not const on stable"
    )]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "0 <= x <= i128::MAX, checked, so -x fits"
    )]
    const fn to_int_any(self, scale: Magnitude) -> i128 {
        if scale.is_rational() {
            return self.to_int(scale);
        }
        let Some(x) = self.to_interval_any(scale) else {
            return 0;
        };
        let x = x.to_int();
        assert!(
            x <= i128::MAX as u128,
            "this literal is too large for its storage"
        );
        #[expect(
            clippy::cast_possible_wrap,
            reason = "x is checked to be at most i128::MAX just above"
        )]
        let x = x as i128;
        if self.num < 0 { -x } else { x }
    }
}

/// The scale of a literal in unit `S` against stored values of `P · U`, `U · P / S`, or a compile error for units
/// that don't convert ([Fixed::convert]'s rules).
const fn scale_in<S: Unit, U: Unit, P: FixedScale>() -> Magnitude {
    if !S::DIM.same(U::DIM) {
        crate::conv_error::dim_mismatch::<S, U>("a literal's unit");
    }
    if !S::KIND.same(U::KIND) {
        crate::conv_error::kind_mismatch::<S, U>("a literal's unit");
    }
    if S::EXPR.has_offset() || U::EXPR.has_offset() {
        crate::conv_error::has_offset::<S, U>("a literal's unit");
    }
    let (Some(from), Some(to)) = (S::EXPR.magnitude(), U::EXPR.magnitude()) else {
        crate::conv_error::no_exact_factor::<S, U>("a literal's unit")
    };
    to.mul(P::SCALE).div(from)
}

/// `a · b`, or a compile error: the scale and the literal together need more than 128 bits.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (a panic at run time), as documented: a fraction past u128"
)]
const fn mul(a: u128, b: u128) -> u128 {
    match a.checked_mul(b) {
        Some(x) => x,
        None => panic!(
            "a literal too long for its scale: the exact stored value needs more than 128 bits"
        ),
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

/// A decimal literal as a type, `NUM · 10^EXP`: what [fixed!](crate::fixed) names, so the scale and storage of the
/// [Fixed] can come from the context. A pair of them is a [Complex]'s.
#[derive(Debug)]
pub struct Literal<const NUM: i64, const EXP: i16>;

/// A decimal literal in the unit `S` as a type, `NUM · 10^EXP` of `S`: what `fixed!(17.7 in Decibel)` names.
#[derive(Debug)]
pub struct LiteralIn<const NUM: i64, const EXP: i16, S>(core::marker::PhantomData<S>);

/// The stored value of literal `L` as an associated const, worked out at compile time ([Fixed::from_decimal]): what
/// [fixed!](crate::fixed) reads, for every [Fixed] with an integer or float storage and every [Complex].
pub trait AtLiteral<L>: Sized {
    /// The stored value nearest to literal `L`.
    const AT: Self;
}

// `fixed!(100 m/s)` reads its unit through q!'s reader: the type of the literal where the feature `q` is on, an error
// naming the feature where it is off.

#[cfg(feature = "q")]
#[doc(hidden)]
#[macro_export]
macro_rules! __fixed_unit {
    ($($t:tt)*) => { $crate::__macros::fixed!($crate; $($t)*) };
}

#[cfg(not(feature = "q"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __fixed_unit {
    ($($t:tt)*) => {
        ::core::compile_error!(
            "fixed!(v unit) reads its unit symbols with exact_units' feature `q`; without it write fixed!(v in Unit)"
        )
    };
}

/// A constant of the [Fixed] (or, of two literals, the [Complex]) the context names, from a decimal literal in units
/// of its unit, rounded to the nearest stored value at compile time: see the [module](crate::literal).
///
/// ```
/// use exact_units::{CQ15, Fixed, fixed, prefix::Pow2, units::Ratio};
/// const GAIN: Fixed<Ratio, Pow2<-15>, i16> = fixed!(0.7071); // 23170.25..
/// assert_eq!(GAIN.get(), 23170);
/// const Z: CQ15 = fixed!(0.5, -0.25);
/// assert_eq!(Z.get(), (16384, -8192));
/// ```
///
/// A literal in another unit of the same dimension and kind is written `in` it:
///
/// ```
/// # #[cfg(feature = "angle")] {
/// use exact_units::{Fixed, fixed, prefix::Micro, units::{Degree, Radian}};
/// const RIGHT: Fixed<Radian, Micro, i32> = fixed!(90 in Degree);
/// assert_eq!(RIGHT.get(), 1_570_796); // π/2 = 1.5707963..
/// # }
/// ```
///
/// That other unit can be named the way [q!](crate::q) reads one, symbols and all, rather than a type path:
///
/// ```
/// # #[cfg(feature = "mechanical")] {
/// use exact_units::{Fixed, fixed, prefix::Centi, units::Speed};
/// const LIMIT: Fixed<Speed, Centi, i16> = fixed!(100 m/s);
/// assert_eq!(LIMIT.get(), 10_000);
/// const OTHER: Fixed<Speed, Centi, i16> = fixed!(359.9 km/h); // 99.9722.. m/s, rounded to even
/// assert_eq!(OTHER.get(), 9_997);
/// # }
/// ```
///
/// A symbol whose unit doesn't convert to the context's (a length is not a speed) doesn't compile either:
///
/// ```compile_fail
/// # #[cfg(feature = "mechanical")] {
/// use exact_units::{Fixed, fixed, prefix::Centi, units::Speed};
/// const X: Fixed<Speed, Centi, i16> = fixed!(100 m);
/// # }
/// ```
///
/// A value that doesn't fit its storage doesn't compile, nor does a negative one of an unsigned storage, one outside
/// the bounds or one of a scale that isn't a fraction of integers:
///
/// ```compile_fail
/// use exact_units::{Fixed, fixed, prefix::Pow2, units::Ratio};
/// const ONE: Fixed<Ratio, Pow2<-15>, i16> = fixed!(1.0); // 32768 is one past i16::MAX
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, fixed, prefix::Milli, units::Ratio};
/// const X: Fixed<Ratio, Milli, u16> = fixed!(-0.001);
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, Range, fixed, prefix::One, units::Ratio};
/// const X: Fixed<Ratio, One, i8, Range<0, 10>> = fixed!(11);
/// ```
///
/// ```compile_fail
/// use exact_units::{Fixed, fixed, prefix::One, units::Ratio};
/// const X: Fixed<Ratio, One, f32> = fixed!(0.5f32); // the storage comes from the context
/// ```
///
/// ```compile_fail
/// exact_units::scale! { struct PiQ15 = "", Exact(1 / 32768, pi 1); }
/// use exact_units::{Fixed, fixed, units::Ratio};
/// const X: Fixed<Ratio, PiQ15, i16> = fixed!(0.5);
/// ```
#[macro_export]
macro_rules! fixed {
    ($re:literal, $im:literal $(,)?) => {
        <_ as $crate::literal::AtLiteral<(
            $crate::literal::Literal<
                { $crate::literal::Decimal::parse(stringify!($re)).num() },
                { $crate::literal::Decimal::parse(stringify!($re)).exp() },
            >,
            $crate::literal::Literal<
                { $crate::literal::Decimal::parse(stringify!($im)).num() },
                { $crate::literal::Decimal::parse(stringify!($im)).exp() },
            >,
        )>>::AT
    };
    ($value:literal in $unit:ty) => {
        <_ as $crate::literal::AtLiteral<
            $crate::literal::LiteralIn<
                { $crate::literal::Decimal::parse(stringify!($value)).num() },
                { $crate::literal::Decimal::parse(stringify!($value)).exp() },
                $unit,
            >,
        >>::AT
    };
    ($value:literal $($unit:tt)+) => {
        <_ as $crate::literal::AtLiteral<$crate::__fixed_unit!($value $($unit)+)>>::AT
    };
    ($value:literal) => {
        <_ as $crate::literal::AtLiteral<
            $crate::literal::Literal<
                { $crate::literal::Decimal::parse(stringify!($value)).num() },
                { $crate::literal::Decimal::parse(stringify!($value)).exp() },
            >,
        >>::AT
    };
}

/// `from_decimal` per storage: the integers through `bounded`, the floats checked against `B` here.
macro_rules! from_decimal {
    (int $($ty:ident)*) => {$(
        impl<U, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The stored value nearest to `x` in units of `U`, ties to even, at compile time: see the
            /// [module](crate::literal). A compile error in `const` code (a panic at run time) for a value that
            /// doesn't fit `T` or `B`, or a scale that isn't a fraction of integers.
            pub const fn from_decimal(x: Decimal) -> Self {
                Self::of_literal(x.to_int(P::SCALE))
            }

            /// `v` in `T` and `B`, or a compile error.
            #[expect(clippy::allow_attributes, reason = "the casts differ per storage: an #[expect] is unfulfilled where one is exact")]
            #[expect(clippy::as_conversions, reason = "const fn: From is not const on stable")]
            const fn of_literal(v: i128) -> Self {
                if v < $ty::MIN as i128 {
                    panic!("this literal is below the range of its storage");
                }
                if v > $ty::MAX as i128 {
                    panic!("this literal is too large for its storage");
                }
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "v is checked to be in the storage's range just above"
                )]
                let v = v as $ty;
                Self::bounded(v)
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The stored value nearest to `x` in units of `S`, ties to even, at compile time: what
            /// `fixed!(17.7 in Decibel)` reads, a literal in another unit of the same dimension and kind. A factor
            /// that isn't a fraction of integers (dB in Np, ° in rad) is rounded from an interval around the exact
            /// value. A compile error in `const` code (a panic at run time) for units that don't convert, a value
            /// that doesn't fit `T` or `B`, or one too close to halfway between two stored values to decide.
            pub const fn from_decimal_in<S: Unit>(x: Decimal) -> Self {
                Self::of_literal(x.to_int_any(scale_in::<S, U, P>()))
            }
        }

        impl<U, P: FixedScale, B: Bounds<$ty>, const N: i64, const E: i16> AtLiteral<Literal<N, E>>
            for Fixed<U, P, $ty, B>
        {
            const AT: Self = Self::from_decimal(Decimal::new(N, E));
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, S: Unit, const N: i64, const E: i16>
            AtLiteral<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            const AT: Self = Self::from_decimal_in::<S>(Decimal::new(N, E));
        }
    )*};
    (float $($ty:ident $to:ident)*) => {$(
        impl<U, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The float nearest to `x` in units of `U`, ties to even, at compile time: see the
            /// [module](crate::literal). A compile error in `const` code (a panic at run time) for a value outside
            /// `B` or the float's normal range, or a scale that isn't a fraction of integers.
            pub const fn from_decimal(x: Decimal) -> Self {
                Self::of_literal(x, x.to_interval(P::SCALE))
            }

            /// `±|x|` from its interval, checked against `B`.
            const fn of_literal(x: Decimal, abs: Option<Interval>) -> Self {
                let v = match abs {
                    Some(i) => i.$to(),
                    None => 0.0,
                };
                let v = if x.num() < 0 { -v } else { v };
                if let Some(min) = B::MIN
                    && v < min
                {
                    panic!("value below its bound");
                }
                if let Some(max) = B::MAX
                    && v > max
                {
                    panic!("value above its bound");
                }
                Self::new_unchecked(v)
            }
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>> Fixed<U, P, $ty, B> {
            /// The float nearest to `x` in units of `S`, ties to even, at compile time: what
            /// `fixed!(17.7 in Decibel)` reads. A compile error in `const` code (a panic at run time) for units that
            /// don't convert, a value outside `B` or the float's normal range, or one too close to halfway between
            /// two floats to decide.
            pub const fn from_decimal_in<S: Unit>(x: Decimal) -> Self {
                Self::of_literal(x, x.to_interval_any(scale_in::<S, U, P>()))
            }
        }

        impl<U, P: FixedScale, B: Bounds<$ty>, const N: i64, const E: i16> AtLiteral<Literal<N, E>>
            for Fixed<U, P, $ty, B>
        {
            const AT: Self = Self::from_decimal(Decimal::new(N, E));
        }

        impl<U: Unit, P: FixedScale, B: Bounds<$ty>, S: Unit, const N: i64, const E: i16>
            AtLiteral<LiteralIn<N, E, S>> for Fixed<U, P, $ty, B>
        {
            const AT: Self = Self::from_decimal_in::<S>(Decimal::new(N, E));
        }
    )*};
}

from_decimal!(int u8 u16 u32 u64 i8 i16 i32 i64);

from_decimal!(float f32 to_f32 f64 to_f64);

/// A [Complex] of two literals, each part as [Fixed::from_decimal] has it.
macro_rules! complex_literal {
    ($($ty:ident)*) => {$(
        impl<U, P: FixedScale, B: Unlimited<$ty>, const N1: i64, const E1: i16, const N2: i64, const E2: i16>
            AtLiteral<(Literal<N1, E1>, Literal<N2, E2>)> for Complex<U, P, $ty, B>
        {
            const AT: Self = Complex::of(
                Fixed::<U, P, $ty>::from_decimal(Decimal::new(N1, E1)).get(),
                Fixed::<U, P, $ty>::from_decimal(Decimal::new(N2, E2)).get(),
            );
        }
    )*};
}

complex_literal!(i8 i16 i32 i64 f32 f64);

#[cfg(feature = "q")]
pub mod point;
#[cfg(feature = "q")]
pub mod quantity;
#[cfg(feature = "q")]
pub use point::{LiteralAt, PointUnit, Steps};
#[cfg(feature = "q")]
#[doc(hidden)]
pub use quantity::Unitless;

#[cfg(test)]
mod tests;
