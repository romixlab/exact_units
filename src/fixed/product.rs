//! Products and quotients of two [Fixed] values: `a * b` is a `Fixed<Times<U1, U2>, Product<P1, P2>, T>`, `a / b` a
//! `Fixed<Per<U1, U2>, Quotient<P1, P2>, T>`.
//!
//! ```
//! # #[cfg(feature = "electrical")] {
//! use exact_units::{Fixed, prefix::{Micro, Milli}, units::{Ampere, Volt, Watt}};
//!
//! let v = Fixed::<Volt, Milli, i32>::new(3300);
//! let i = Fixed::<Ampere, Milli, i32>::new(250);
//! // 3300 mV · 250 mA is 825 000 of mV·mA, a scale of 10⁻⁶ exactly
//! let p = v * i;
//! assert_eq!(p.get(), 825_000);
//! assert_eq!(p.to_string(), "825000 µ(V·A)");
//! // V·A is a watt by convert, here free: the factor is 1
//! assert_eq!(p.convert::<Watt, Micro, i32>(), Some(Fixed::new(825_000)));
//! # }
//! ```
//!
//! How each question is settled, and why:
//!
//! - **The written form is kept.** The unit of `a * b` is [Times]`<U1, U2>` and of `a / b` [Per]`<U1, U2>`, never
//!   normalized: V/V · V stays `Times<Per<Volt, Volt>, Volt>`. The named unit is a [convert](Fixed::convert) away
//!   (`V·A` to [Watt](crate::units::Watt), V/V · V to a volt), which checks the dimension and the [kind](crate::kind)
//!   at compile time. The kind of a product or quotient follows the rules of the kind module (mp-units'): V/V · V is
//!   a plain voltage and converts to `Volt`, V/V · V/V stays a voltage gain, V/A · A is a plain voltage, Ω · A too.
//! - **The scale is exact, as a type.** Stable Rust can't compute `Pow10<{A + B}>` in a type, so the scale of a
//!   product is the marker [Product]`<P1, P2>` and of a quotient [Quotient]`<P1, P2>`, whose
//!   [SCALE](FixedScale::SCALE) is the exact [Magnitude] product (or quotient) of the two, worked out at compile
//!   time: named prefixes, [Pow10](prefix::Pow10), [Pow2](prefix::Pow2), [FullScale](prefix::FullScale), fractions
//!   and powers of π all multiply exactly. Equal scales are the same scale however spelled, so `Product<Milli,
//!   Milli>` to [Micro](prefix::Micro) is free and `Product<Pow2<-15>, Pow2<-15>>` is `Pow2<-30>`. There is always an
//!   exact scale; the one product that can't be one, more than [MAX_FACTORS](crate::MAX_FACTORS) primes, doesn't
//!   compile where it is written.
//! - **The operator multiplies the stored numbers, nothing else.** `(a · P1) · (b · P2) = (a · b) · (P1 · P2)`, so
//!   `a * b` is one multiply of `T` and `a / b` one division of `T`: no rescale, no float at run time for an integer
//!   storage. A division truncates as `T`'s does (3 V / 2 A is 1 V/A): for a quotient that keeps its digits, and
//!   for a product into another scale or a narrower storage, [times_into](Fixed::times_into) and
//!   [per_into](Fixed::per_into) give the result in a scale `P3` and storage `T3` of your choice, rounded once by
//!   the [Rounding] given (Q15 · Q15 back into Q15 is `times_into::<Pow2<-15>, i16>`).
//! - **Overflow by the policy of the operands**, as `Fixed * T` does: for [Unbounded] `T`'s own (a panic in debug
//!   builds), for [Wrapping] modulo 2^bits, for [Saturating] at the ends of `T`, and the result keeps the policy.
//!   Two different policies don't multiply (`with_overflow` one of them first). The other policies are named:
//!   [checked_times](Fixed::checked_times), [saturating_times](Fixed::saturating_times),
//!   [wrapping_times](Fixed::wrapping_times) and their `_per` twins, for operands of any bounds. The rounding forms
//!   take an [Overflow] ([times_requantize](Fixed::times_requantize)) or check ([times_into](Fixed::times_into)).
//! - **Bounds of the result are [Unbounded].** A range of the result would need arithmetic on const parameters in
//!   the type. Two [Range]s still multiply with `*`: their corners are checked at compile time to fit `T`, so the
//!   product can't overflow, and a pair whose product may not fit doesn't compile. Any other bounded operand takes
//!   `checked_times` or `.unbounded()`.
//! - **One storage.** `a` and `b` have the same `T`; mixed storages don't multiply, convert one first (`to`), so
//!   the width the multiply runs in is always the one written.
//! - **Vectors only.** A [Point](crate::Point) has no product (twice 20 °C is not 40 °C), and a unit with an offset
//!   scale (°C as a `Fixed`) doesn't multiply either, like in the product table: take its difference as a
//!   kelvin.
//!
//!
//! What it costs on a microcontroller, measured on 29 October 2026 with `qemu_icount` (from WireWeaver's
//! `examples_mcu`) on a Cortex-M3 (`thumbv7m-none-eabi`, QEMU `lm3s6965evb`), fat LTO and one codegen unit at
//! `opt-level = "z"` and `3`, 64 values per run, each case against a plain twin that gives the same bytes:
//!
//! | Case | `z` plain | `z` `Fixed` | `3` plain | `3` `Fixed` |
//! |---|---|---|---|---|
//! | mV · mA in µW, `i32` (`a * b`) | 652 | 652 | 270 | 270 |
//! | mV · mA into mW, `i32`, nearest even, checked (`times_into`) | 6973 | 7188 | 6933 | 6869 |
//! | Q15 · Q15 into Q15, floor, saturating (`times_requantize`) | 975 | 975 | 698 | 698 |
//!
//! The product and the Q15 multiply are the same instructions as the plain code (identical at `3`, at `z` apart
//! from the offsets of calls); the rounded mW is within 3 %, most of both being the 64-bit division the M3 calls
//! `__aeabi_ldivmod` for.
//!
//! [Times]: crate::units::Times
//! [Per]: crate::units::Per
//!
//! Mixed storages don't multiply:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Ampere, Volt}};
//! let _ = Fixed::<Volt, One, i32>::new(1) * Fixed::<Ampere, One, i16>::new(1);
//! ```
//!
//! nor two overflow policies:
//!
//! ```compile_fail
//! use exact_units::{Fixed, Saturating, Wrapping, prefix::One, units::{Ampere, Volt}};
//! let _ = Fixed::<Volt, One, i32, Wrapping>::of(1) * Fixed::<Ampere, One, i32, Saturating>::of(1);
//! ```
//!
//! nor a bounded value with `*` unless both are ranges (`checked_times` is the step):
//!
//! ```compile_fail
//! use exact_units::{Fixed, Range, prefix::One, units::{Ampere, Volt}};
//! let _ = Fixed::<Volt, One, i32, Range<0, 10>>::bounded(1) * Fixed::<Ampere, One, i32>::new(1);
//! ```
//!
//! nor two ranges whose product may not fit the storage:
//!
//! ```compile_fail
//! use exact_units::{Fixed, Range, prefix::One, units::{Ampere, Volt}};
//! type V = Fixed<Volt, One, i16, Range<0, 1000>>;
//! type A = Fixed<Ampere, One, i16, Range<0, 1000>>;
//! let _ = V::bounded(1) * A::bounded(1); // 10⁶ doesn't fit an i16
//! ```
//!
//! nor a point:
//!
//! ```compile_fail
//! use exact_units::{Fixed, Point, origins::Celsius, prefix::Centi, units::{Kelvin, Ratio}};
//! let t = Point::<Celsius, Fixed<Kelvin, Centi, i32>>::from_raw(2000);
//! let _ = t * Fixed::<Ratio, Centi, i32>::new(200);
//! ```
//!
//! nor a unit with an offset:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{DegreeCelsius, Ratio}};
//! let _ = Fixed::<DegreeCelsius, One, i32>::new(20) * Fixed::<Ratio, One, i32>::new(2);
//! ```
//!
//! A product of two voltage gains is a voltage gain, not a plain ratio:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Per, Ratio, Volt}};
//! let g = Fixed::<Per<Volt, Volt>, One, i32>::new(2);
//! let _ = (g * g).convert::<Ratio, One, i32>();
//! ```
//!
//! and a resistance times a gain is still a resistance, not a transimpedance:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::One, units::{Ampere, Ohm, Per, Volt}};
//! let g = Fixed::<Per<Volt, Volt>, One, i32>::new(2);
//! let _ = (Fixed::<Ohm, One, i32>::new(50) * g).convert::<Per<Volt, Ampere>, One, i32>();
//! ```
//!
//! A product of more primes than a [Magnitude] holds has no exact scale and doesn't compile:
//!
//! ```compile_fail
//! use exact_units::{Fixed, prefix::Rational, units::{Ampere, Volt}};
//! // 3·7·11·13·17·19·23 and 29·31·37·41·43·47: 13 primes
//! let a = Fixed::<Volt, Rational<22309287, 1>, i32>::new(1);
//! let b = Fixed::<Ampere, Rational<2756205443, 1>, i32>::new(1);
//! let _ = a * b;
//! ```
//!
//! An integer result through a factor that isn't a fraction of integers (a power of π) doesn't compile; through a
//! float it does:
//!
//! ```compile_fail
//! use exact_units::{Fixed, Rounding, prefix::One, units::{Radian, Ratio}};
//! exact_units::scale! {
//!     /// Steps of π rad.
//!     struct PiSteps = "", Exact(1 / 1, pi 1);
//! }
//! let a = Fixed::<Radian, PiSteps, i32>::new(1);
//! let _: Option<Fixed<_, One, i32>> = a.times_into(Fixed::<Ratio, One, i32>::new(2), Rounding::Floor);
//! ```

use super::rounding::{self, Overflow, Rounding};
use super::{Bounds, Fixed, FixedScale, Range, Repr, Saturating, Storage, Unbounded, Wrapping};
use crate::Magnitude;
use crate::prefix::{Product, Quotient};
use crate::unit::Unit;
use crate::units::{Per, Times};
use core::ops::{Div, Mul};

/// What every product and quotient checks at compile time: no unit with an offset, and an exact scale.
const fn check<U1: Unit, U2: Unit, P1: FixedScale, P2: FixedScale>(per: bool) {
    assert!(
        !U1::EXPR.has_offset() && !U2::EXPR.has_offset(),
        "a unit with an offset scale (°C) doesn't multiply or divide: take its difference in kelvin"
    );
    let scale = if per {
        Quotient::<P1, P2>::SCALE
    } else {
        Product::<P1, P2>::SCALE
    };
    // read, so a product without an exact scale is an error where it is written
    let _ = scale.as_pow10();
}

/// `*` and `/` of two values of one overflow policy, which the result keeps.
macro_rules! policy {
    ($($b:ident $mul:ident $div:ident;)*) => {$(
        impl<U1: Unit, P1: FixedScale, U2: Unit, P2: FixedScale, T: Storage> Mul<Fixed<U2, P2, T, $b>>
            for Fixed<U1, P1, T, $b>
        {
            type Output = Fixed<Times<U1, U2>, Product<P1, P2>, T, $b>;
            #[inline(always)]
            fn mul(self, rhs: Fixed<U2, P2, T, $b>) -> Self::Output {
                const { check::<U1, U2, P1, P2>(false) };
                Fixed::new_unchecked(self.value.$mul(rhs.value))
            }
        }

        /// Division by 0 panics as for `T`; only `MIN / -1` of a signed `T` overflows.
        impl<U1: Unit, P1: FixedScale, U2: Unit, P2: FixedScale, T: Storage> Div<Fixed<U2, P2, T, $b>>
            for Fixed<U1, P1, T, $b>
        {
            type Output = Fixed<Per<U1, U2>, Quotient<P1, P2>, T, $b>;
            #[inline(always)]
            fn div(self, rhs: Fixed<U2, P2, T, $b>) -> Self::Output {
                const { check::<U1, U2, P1, P2>(true) };
                Fixed::new_unchecked(self.value.$div(rhs.value))
            }
        }
    )*};
}

policy! {
    Unbounded mul div;
    Wrapping wrapping_mul wrapping_div;
    Saturating saturating_mul saturating_div;
}

/// The product of two ranges, which can't overflow: the corners of the ranges are checked at compile time to fit
/// `T`, else it doesn't compile. The result is [Unbounded].
impl<U1, P1, U2, P2, T, const A0: i64, const A1: i64, const B0: i64, const B1: i64>
    Mul<Fixed<U2, P2, T, Range<B0, B1>>> for Fixed<U1, P1, T, Range<A0, A1>>
where
    U1: Unit,
    P1: FixedScale,
    U2: Unit,
    P2: FixedScale,
    T: Storage,
    Range<A0, A1>: Bounds<T>,
    Range<B0, B1>: Bounds<T>,
{
    type Output = Fixed<Times<U1, U2>, Product<P1, P2>, T>;
    #[inline(always)]
    fn mul(self, rhs: Fixed<U2, P2, T, Range<B0, B1>>) -> Self::Output {
        const {
            check::<U1, U2, P1, P2>(false);
            proven_product::<T>(A0, A1, B0, B1)
        };
        // proven in range: wrapping is the plain multiply
        Fixed::new(self.value.wrapping_mul(rhs.value))
    }
}

/// Every product of `a0..=a1` and `b0..=b1` fits `T`, else a compile error.
const fn proven_product<T: Storage>(a0: i64, a1: i64, b0: i64, b1: i64) {
    let c = [
        a0 as i128 * b0 as i128,
        a0 as i128 * b1 as i128,
        a1 as i128 * b0 as i128,
        a1 as i128 * b1 as i128,
    ];
    let (mut lo, mut hi) = (c[0], c[0]);
    let mut i = 1;
    while i < 4 {
        if c[i] < lo {
            lo = c[i];
        }
        if c[i] > hi {
            hi = c[i];
        }
        i += 1;
    }
    assert!(
        lo >= T::MIN_INT && hi <= T::MAX_INT,
        "the bounds don't prove that the product fits the storage: widen the storage, narrow the bounds, or use \
         checked_times or times_into"
    );
}

impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T>> Fixed<U, P, T, B> {
    /// `self · rhs` in the scale [Product]`<P, P2>`, `None` on overflow of `T`. For operands of any bounds; the
    /// result is [Unbounded].
    #[inline(always)]
    #[allow(clippy::type_complexity)] // the written form of the result is the point
    pub fn checked_times<U2: Unit, P2: FixedScale, B2: Bounds<T>>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
    ) -> Option<Fixed<Times<U, U2>, Product<P, P2>, T>> {
        const { check::<U, U2, P, P2>(false) };
        self.value.checked_mul(rhs.value).map(Fixed::new)
    }

    /// `self · rhs` in the scale [Product]`<P, P2>`, at the nearest end of `T` where it doesn't fit.
    #[inline(always)]
    pub fn saturating_times<U2: Unit, P2: FixedScale, B2: Bounds<T>>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
    ) -> Fixed<Times<U, U2>, Product<P, P2>, T> {
        const { check::<U, U2, P, P2>(false) };
        Fixed::new(self.value.saturating_mul(rhs.value))
    }

    /// `self · rhs` in the scale [Product]`<P, P2>`, modulo 2^bits of `T`.
    #[inline(always)]
    pub fn wrapping_times<U2: Unit, P2: FixedScale, B2: Bounds<T>>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
    ) -> Fixed<Times<U, U2>, Product<P, P2>, T> {
        const { check::<U, U2, P, P2>(false) };
        Fixed::new(self.value.wrapping_mul(rhs.value))
    }

    /// `self / rhs` in the scale [Quotient]`<P, P2>`, truncated as `T` divides; `None` for a zero divisor (of an
    /// integer `T`) or on overflow (`MIN / -1`).
    #[inline(always)]
    #[allow(clippy::type_complexity)] // the written form of the result is the point
    pub fn checked_per<U2: Unit, P2: FixedScale, B2: Bounds<T>>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
    ) -> Option<Fixed<Per<U, U2>, Quotient<P, P2>, T>> {
        const { check::<U, U2, P, P2>(true) };
        self.value.checked_div(rhs.value).map(Fixed::new)
    }

    /// `self / rhs` in the scale [Quotient]`<P, P2>`, `MIN / -1` at `MAX`; a zero divisor panics as for `T`.
    #[inline(always)]
    pub fn saturating_per<U2: Unit, P2: FixedScale, B2: Bounds<T>>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
    ) -> Fixed<Per<U, U2>, Quotient<P, P2>, T> {
        const { check::<U, U2, P, P2>(true) };
        Fixed::new(self.value.saturating_div(rhs.value))
    }

    /// `self / rhs` in the scale [Quotient]`<P, P2>`, `MIN / -1` at `MIN`; a zero divisor panics as for `T`.
    #[inline(always)]
    pub fn wrapping_per<U2: Unit, P2: FixedScale, B2: Bounds<T>>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
    ) -> Fixed<Per<U, U2>, Quotient<P, P2>, T> {
        const { check::<U, U2, P, P2>(true) };
        Fixed::new(self.value.wrapping_div(rhs.value))
    }

    /// `self · rhs` in the scale `P3` and storage `T3`, rounded once by `r`; `None` where it doesn't fit `T3`. The
    /// scale and storage are usually given by the binding, `let p: Fixed<_, Milli, i32> = ..`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, Q15, Rounding, prefix::{Milli, Pow2}, units::{Ampere, Ratio, Volt}};
    ///
    /// // 3.3 V · 1.5 A in mW: the product of mV and mA is µW, rounded once into mW of an i32
    /// let v = Fixed::<Volt, Milli, i32>::new(3301);
    /// let i = Fixed::<Ampere, Milli, i32>::new(1500);
    /// let p: Fixed<_, Milli, i32> = v.times_into(i, Rounding::NearestEven).unwrap();
    /// assert_eq!(p.get(), 4952); // 4951.5 mW, to even
    ///
    /// // a Q15 multiply: 0.5 · 0.5 is 0.25, back in steps of 2⁻¹⁵
    /// let half: Q15 = Fixed::of(16384);
    /// let q: Fixed<_, Pow2<-15>, i16> = half.times_into(half, Rounding::Floor).unwrap();
    /// assert_eq!(q.get(), 8192);
    /// # }
    /// ```
    ///
    /// The factor `P · P2 / P3` is worked out at compile time. Between integer storages it must be a fraction of
    /// integers, and `|T|² · num` must fit 128 bits (else this doesn't compile); where it also fits 64 bits and
    /// `T3` has up to 32, the work is a 64-bit multiply and one division by a constant (none for a factor of 1).
    /// With a float on either side it is scaled in `f64`.
    #[inline(always)]
    pub fn times_into<U2: Unit, P2: FixedScale, B2: Bounds<T>, P3: FixedScale, T3: Storage>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
        r: Rounding,
    ) -> Option<Fixed<Times<U, U2>, P3, T3>> {
        self.times_requantize(rhs, r, Overflow::Checked)
    }

    /// As [times_into](Self::times_into), settled by `o` where it doesn't fit `T3`; `None` only for
    /// [Overflow::Checked].
    #[inline(always)]
    pub fn times_requantize<
        U2: Unit,
        P2: FixedScale,
        B2: Bounds<T>,
        P3: FixedScale,
        T3: Storage,
    >(
        self,
        rhs: Fixed<U2, P2, T, B2>,
        r: Rounding,
        o: Overflow,
    ) -> Option<Fixed<Times<U, U2>, P3, T3>> {
        let q = const {
            check::<U, U2, P, P2>(false);
            Requant::of::<T, T3>(Product::<P, P2>::SCALE.div(P3::SCALE), false)
        };
        q.times(self.value, rhs.value, r, o).map(Fixed::new)
    }

    /// `self / rhs` in the scale `P3` and storage `T3`, rounded once by `r`: the quotient with the digits `P3`
    /// keeps, where `/` truncates in [Quotient]`<P, P2>`. `None` for a zero divisor or where it doesn't fit `T3`.
    ///
    /// ```
    /// # #[cfg(feature = "electrical")] {
    /// use exact_units::{Fixed, Rounding, prefix::{Milli, One}, units::{Ampere, Ohm, Volt}};
    ///
    /// let v = Fixed::<Volt, One, i32>::new(3);
    /// let i = Fixed::<Ampere, One, i32>::new(2);
    /// assert_eq!((v / i).get(), 1); // truncated, as i32 divides
    /// let r: Fixed<_, Milli, i32> = v.per_into(i, Rounding::NearestEven).unwrap();
    /// assert_eq!(r.get(), 1500);
    /// // V/A is a transimpedance; a resistance by its named step
    /// let r: Fixed<Ohm, Milli, i32> = r.as_resistance();
    /// assert_eq!(r.get(), 1500);
    /// # }
    /// ```
    ///
    /// The factor `P / (P2 · P3)` is worked out at compile time, with the limits of [times_into](Self::times_into)
    /// on `|T| · num` and `|T| · den`.
    #[inline(always)]
    pub fn per_into<U2: Unit, P2: FixedScale, B2: Bounds<T>, P3: FixedScale, T3: Storage>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
        r: Rounding,
    ) -> Option<Fixed<Per<U, U2>, P3, T3>> {
        self.per_requantize(rhs, r, Overflow::Checked)
    }

    /// As [per_into](Self::per_into), settled by `o` where it doesn't fit `T3`; `None` for a zero divisor, else only
    /// for [Overflow::Checked].
    #[inline(always)]
    pub fn per_requantize<U2: Unit, P2: FixedScale, B2: Bounds<T>, P3: FixedScale, T3: Storage>(
        self,
        rhs: Fixed<U2, P2, T, B2>,
        r: Rounding,
        o: Overflow,
    ) -> Option<Fixed<Per<U, U2>, P3, T3>> {
        let q = const {
            check::<U, U2, P, P2>(true);
            Requant::of::<T, T3>(Quotient::<P, P2>::SCALE.div(P3::SCALE), true)
        };
        q.per(self.value, rhs.value, r, o).map(Fixed::new)
    }
}

/// The factor of a rounding product or quotient into another scale and storage, worked out at compile time:
/// `a · b · num / den` or `a · num / (b · den)`.
#[derive(Copy, Clone)]
struct Requant {
    num: u128,
    den: u128,
    /// Integers whose largest product fits `i64`: the work is in 64 bits.
    narrow: bool,
    /// A float storage on either side: in `f64`, by `num_f / den_f` where both are exact, else by `factor`.
    float: bool,
    exact_f: bool,
    num_f: f64,
    den_f: f64,
    factor: f64,
}

/// `a · b`, `u128::MAX` where it doesn't fit.
const fn mul_or_max(a: u128, b: u128) -> u128 {
    match a.checked_mul(b) {
        Some(x) => x,
        None => u128::MAX,
    }
}

impl Requant {
    const fn of<T: Storage, T3: Storage>(m: Magnitude, per: bool) -> Self {
        let float = T::IS_FLOAT || T3::IS_FLOAT;
        let ratio = m.to_ratio();
        let (num, den) = match ratio {
            Some(r) => r,
            None if float => (1, 1),
            None => panic!(
                "an integer product or quotient rescales by a fraction of integers only: this factor has a power of \
                 π, ln 2 or ln 10 or a half exponent (multiply in a float storage)"
            ),
        };
        let mut narrow = false;
        if !float {
            let max = if T::MAX_INT.unsigned_abs() > T::MIN_INT.unsigned_abs() {
                T::MAX_INT.unsigned_abs()
            } else {
                T::MIN_INT.unsigned_abs()
            };
            // the largest numerator and divisor before the one division
            let (top, bottom) = if per {
                (mul_or_max(max, num), mul_or_max(max, den))
            } else {
                (mul_or_max(mul_or_max(max, max), num), den)
            };
            assert!(
                top <= i128::MAX as u128 && bottom <= i128::MAX as u128,
                "the factor of this product or quotient doesn't fit the widened multiply in 128 bits"
            );
            narrow = top <= i64::MAX as u128 && bottom <= i64::MAX as u128;
        }
        Requant {
            num,
            den,
            narrow,
            float,
            exact_f: ratio.is_some() && num <= 1 << 53 && den <= 1 << 53,
            num_f: num as f64,
            den_f: den as f64,
            factor: m.to_f64(),
        }
    }

    #[inline(always)]
    fn times<T: Storage, T3: Storage>(self, a: T, b: T, r: Rounding, o: Overflow) -> Option<T3> {
        if self.float {
            return self.finish_f64(to_f64(a) * to_f64(b), r, o);
        }
        if self.narrow {
            // |a · b · num| fits i64, checked at compile time
            let p = a.to_i64() * b.to_i64() * self.num as i64;
            let y = if self.den == 1 {
                p
            } else {
                rounding::div_i64(p, self.den as i64, r)
            };
            return settle_narrow(y, o);
        }
        let (Repr::Int(a), Repr::Int(b)) = (a.to_repr(), b.to_repr()) else {
            return None;
        };
        // fits i128, checked at compile time
        let p = a * b * self.num as i128;
        let y = if self.den == 1 {
            p
        } else {
            rounding::div_i128(p, self.den as i128, r)
        };
        rounding::settle(y, o)
    }

    #[inline(always)]
    fn per<T: Storage, T3: Storage>(self, a: T, b: T, r: Rounding, o: Overflow) -> Option<T3> {
        if self.float {
            let d = to_f64(b);
            if d == 0.0 {
                return None;
            }
            return self.finish_f64(to_f64(a) / d, r, o);
        }
        if self.narrow {
            let (mut n, mut d) = (a.to_i64() * self.num as i64, b.to_i64() * self.den as i64);
            if d == 0 {
                return None;
            }
            if d < 0 {
                (n, d) = (-n, -d);
            }
            return settle_narrow(rounding::div_i64(n, d, r), o);
        }
        let (Repr::Int(a), Repr::Int(b)) = (a.to_repr(), b.to_repr()) else {
            return None;
        };
        let (mut n, mut d) = (a * self.num as i128, b * self.den as i128);
        if d == 0 {
            return None;
        }
        if d < 0 {
            (n, d) = (-n, -d);
        }
        rounding::settle(rounding::div_i128(n, d, r), o)
    }

    /// `x` scaled by the factor, into `T3`.
    #[inline(always)]
    fn finish_f64<T3: Storage>(self, x: f64, r: Rounding, o: Overflow) -> Option<T3> {
        let y = if self.exact_f {
            x * self.num_f / self.den_f
        } else {
            x * self.factor
        };
        if T3::IS_FLOAT {
            T3::from_float(y)
        } else {
            rounding::settle_f64(rounding::round_f64(y, r), o)
        }
    }
}

/// A result computed in 64 bits into `T3` by `o`: in 64 bits for a `T3` of up to 32, else through `i128`.
#[inline(always)]
fn settle_narrow<T3: Storage>(y: i64, o: Overflow) -> Option<T3> {
    if T3::BITS <= 32 {
        rounding::settle_i64(y, o)
    } else {
        rounding::settle(y as i128, o)
    }
}

#[inline(always)]
fn to_f64<T: Storage>(x: T) -> f64 {
    match x.to_repr() {
        Repr::Int(i) => i as f64,
        Repr::Float(f) => f,
    }
}

#[cfg(all(test, feature = "electrical", feature = "dimensionless"))]
mod tests;
