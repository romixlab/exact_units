//! [Exact]: a value of a unit known at compile time as an exact fraction chosen at run time.

use crate::fixed::FixedScale;
use crate::tree::{is_ratio, takes_bare_prefix, write_unit};
use crate::units::{Per, Times};
use crate::{Bounds, Fixed, Magnitude, NanFree, Prefix, Repr, Rounding, Storage, Unit};
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::ops::{Add, Div, Mul, Neg, Sub};

/// The number of bits of `$x`, an unsigned `$u`, up to the highest one set, 0 for 0. A Thumb-1 core (`cfg(thumb1)`,
/// set by `build.rs`: Cortex-M0, M0+, M23) has no CLZ, and a count of leading zeros over a whole `u128` is a
/// software count for every 32-bit word; there [bit_length_halved] halves it down to the word that holds the
/// highest bit and counts once. Elsewhere `bit_width` is an instruction or a few.
macro_rules! bit_length {
    ($x:expr, $u:ty) => {{
        #[cfg(thumb1)]
        let n = bit_length_halved!($x, $u);
        #[cfg(not(thumb1))]
        let n = <$u>::bit_width($x);
        n
    }};
}

/// [bit_length] by halving: down to the 32-bit word that holds the highest bit (two steps for a `u128`), then one
/// count of leading zeros in it. Checked against `bit_width` for every bit length on every target (`tests`).
#[cfg_attr(
    not(any(test, thumb1)),
    expect(
        unused_macros,
        reason = "used on Thumb-1 targets only, and by the test that checks it"
    )
)]
macro_rules! bit_length_halved {
    ($x:expr, $u:ty) => {{
        let (mut x, mut width, mut base): ($u, u32, u32) = ($x, <$u>::BITS, 0);
        while width > 32 {
            width /= 2;
            let hi = x >> width;
            if hi != 0 {
                x = hi;
                base = base.saturating_add(width);
            }
        }
        #[expect(
            clippy::allow_attributes,
            reason = "a macro over several widths: the cast narrows a u128 and widens a u8, so an #[expect] is \
                      unfulfilled for one of them"
        )]
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_lossless,
            clippy::as_conversions,
            reason = "x is below 2^32 here, halved down to it; a const fn, where From is not const"
        )]
        let w = x as u32;
        base.saturating_add(w.bit_width())
    }};
}

mod compare;
mod decimal;
mod fraction;
mod text;

/// Not API: the order of a [Fixed]'s and an [Exact]'s magnitudes in their parts, the fast path by bit length and the
/// wide products apart, for the property that checks one against the other (`tests/props/exact.rs`).
#[doc(hidden)]
pub mod order_parts {
    pub use super::compare::{magnitude, magnitude_by_bits, magnitude_wide};
}
#[cfg(feature = "dimensionless")]
mod tolerance;

use decimal::{Expansion, write_fraction};
pub use fraction::{Fraction, Fraction8};
#[cfg(feature = "serde")]
pub(crate) use text::read as read_text;
pub use text::{ParseExactError, ParseFractionError};
pub(crate) use text::{number, scale};
#[cfg(feature = "dimensionless")]
pub use tolerance::{OffTarget, Tolerance};

/// A value of unit `U` as an exact fraction: the unit in the type, the value a [Fraction] (a sign and `u128`s in
/// lowest terms) chosen at run time. For values worked out on a host that no fixed scale holds: an STM32H7 PLL
/// output is ref·(N + FRACN/8192)/(M·P), 519.999755859375 MHz for one, 80/3 MHz for a divider of 3.
///
/// The value is in `U` itself, so there is no scale or storage in the type: `Exact<Hertz>` of 1000/3 is 333.(3) Hz.
/// Arithmetic is exact or refused: `+`, `-`, `*` and `/` give an `Option`, `None` on overflow and for a division by
/// zero, never a rounded value (`?` chains them). A product or quotient of two has its unit as `Fixed`'s does
/// ([Times], [Per]); [convert](Self::convert) names the unit it is.
///
/// ```
/// # #[cfg(all(feature = "time", feature = "dimensionless"))] {
/// use exact_units::{Exact, Fraction, prefix::Mega, units::{Hertz, Ratio, Second}};
///
/// // RM limits as consts: 137.5 MHz
/// const LIMIT: Exact<Hertz> = Exact::new_in::<Mega>(1375, 10);
/// let hse = Exact::<Hertz>::new_in::<Mega>(24, 1);
/// // ref / M · (N + FRACN/8192) / P
/// let n = Fraction::int(173).checked_add(Fraction::new(2730, 8192)).unwrap();
/// let vco = ((hse / 8).unwrap() * n).unwrap();
/// assert_eq!(vco.to_string(), "519.999755859375 MHz");
/// let p = (vco / 3).unwrap();
/// assert_eq!(p.to_string(), "173.333251953125 MHz");
/// assert!(p > LIMIT);
/// // a third of 1000 Hz repeats: the repetend in parentheses, never cut
/// assert_eq!((Exact::<Hertz>::int(1000) / 3).unwrap().to_string(), "333.(3) Hz");
///
/// // Hz · s is a ratio, 1 / Hz a time: the period
/// let t = Exact::<Second>::new(1, 1000);
/// let cycles = (Exact::<Hertz>::int(48_000) * t).unwrap().convert::<Ratio>();
/// assert_eq!(cycles, Some(Exact::int(48)));
/// assert_eq!(hse.recip::<Second>().unwrap().to_string(), "41.(6) ns");
/// # }
/// ```
///
/// From and to [Fixed]: [from_fixed](Self::from_fixed) is exact, [to](Self::to) exact or `None`,
/// [to_rounded](Self::to_rounded) rounds by the mode it is given. An `Exact` and a `Fixed` of one unit compare
/// exactly, across scales.
///
/// ```
/// # #[cfg(feature = "time")] {
/// use exact_units::{Exact, Fixed, Rounding, prefix::{Kilo, One}, units::Hertz};
///
/// let f = Exact::<Hertz>::new(80_000_000, 3);
/// assert_eq!(f.to::<One, u64>(), None);
/// assert_eq!(f.to_rounded::<One, u64>(Rounding::Floor), Some(Fixed::new(26_666_666)));
/// assert!(f < Fixed::<Hertz, Kilo, u32>::new(26_667));
/// assert!(f > Fixed::<Hertz, Kilo, u32>::new(26_666));
/// # }
/// ```
///
/// Shown exactly, with an SI prefix: `{}` picks the largest power of 1000 that leaves at least 1 before the point
/// (a ratio: %, ppm or ppb, the largest that leaves 0.1), `{:#}` writes the unit itself, [in_prefix](Self::in_prefix)
/// the prefix given. The number is the decimal when it ends, else the repetend in parentheses when it is at most
/// 12 digits long (`333.(3) kHz`), else the fraction (`1000/7919 kHz`), as it is where the digits are more than text
/// reads back (38 significant ones). Text in any of these forms reads back
/// (`parse`, `serde::with_unit`). A rounded number is [to_rounded](Self::to_rounded) or [to_f64](Self::to_f64), by
/// name: a format's precision (`{:.3}`) changes nothing.
pub struct Exact<U> {
    value: Fraction,
    _unit: PhantomData<U>,
}

/// `?` for `Option` in `const` code.
macro_rules! tri {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return None,
        }
    };
}

impl<U> Exact<U> {
    /// 0.
    pub const ZERO: Self = Self::of(Fraction::ZERO);

    /// The value `x` of `U`.
    #[must_use]
    pub const fn of(x: Fraction) -> Self {
        Self {
            value: x,
            _unit: PhantomData,
        }
    }

    /// `num / den` of `U`. A zero `den` panics: in a `const` item a compile error, as for [Fraction::new].
    #[must_use]
    pub const fn new(num: i128, den: u128) -> Self {
        Self::of(Fraction::new(num, den))
    }

    /// `num / den` of `U`, `None` for a zero `den`.
    #[must_use]
    pub const fn checked_new(num: i128, den: u128) -> Option<Self> {
        Some(Self::of(tri!(Fraction::checked_new(num, den))))
    }

    /// The whole number `n` of `U`.
    #[must_use]
    pub const fn int(n: i128) -> Self {
        Self::of(Fraction::int(n))
    }

    /// `num / den` steps of the scale `P`: `Exact::<Hertz>::new_in::<Mega>(1375, 10)` is 137.5 MHz. A zero `den`, a
    /// value past `u128` or a scale that isn't a fraction (π) panics: in a `const` item a compile error.
    #[must_use]
    #[expect(
        clippy::panic,
        reason = "a compile error in const code, as documented; checked_new_in is the run-time form"
    )]
    pub const fn new_in<P: FixedScale>(num: i128, den: u128) -> Self {
        match Self::checked_new_in::<P>(num, den) {
            Some(x) => x,
            None => {
                panic!("Exact::new_in: a zero denominator, a value past u128 or a scale with π")
            }
        }
    }

    /// `num / den` steps of the scale `P`, `None` for a zero `den`, a value past `u128` or a scale that isn't a
    /// fraction.
    #[must_use]
    pub const fn checked_new_in<P: FixedScale>(num: i128, den: u128) -> Option<Self> {
        Self::from_scaled(tri!(Fraction::checked_new(num, den)), &P::SCALE)
    }

    /// `x · scale` of `U`: a number with a run-time scale. `None` for a scale with π, ln 2, ln 10 or a half exponent,
    /// and past `u128`.
    #[must_use]
    pub const fn from_scaled(x: Fraction, scale: &Magnitude) -> Option<Self> {
        let s = tri!(Fraction::from_magnitude(scale));
        Some(Self::of(tri!(x.checked_mul(s))))
    }

    /// `x` in the prefix `p` (`24` of [Prefix::Mega] is 24 MHz on hertz), `None` past `u128`.
    #[must_use]
    pub fn from_prefix(x: Fraction, p: Prefix) -> Option<Self> {
        Self::from_scaled(x, &p.magnitude()?)
    }

    /// The value, a [Fraction] of `U`.
    #[must_use]
    pub const fn get(self) -> Fraction {
        self.value
    }

    /// 0.
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.value.is_zero()
    }

    /// Below zero.
    #[must_use]
    pub const fn is_negative(self) -> bool {
        self.value.is_negative()
    }

    /// `|self|`.
    #[must_use]
    pub const fn abs(self) -> Self {
        Self::of(self.value.abs())
    }

    /// `self + rhs`, `None` only when the result doesn't fit ([Fraction::checked_add]).
    #[must_use]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        Some(Self::of(tri!(self.value.checked_add(rhs.value))))
    }

    /// `self - rhs`, `None` only when the result doesn't fit.
    #[must_use]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        Some(Self::of(tri!(self.value.checked_sub(rhs.value))))
    }

    /// `self · k`, `None` where the result doesn't fit.
    #[must_use]
    pub const fn checked_scale(self, k: Fraction) -> Option<Self> {
        Some(Self::of(tri!(self.value.checked_mul(k))))
    }

    /// `self / k`, `None` for 0 and where the result doesn't fit.
    #[must_use]
    pub const fn checked_unscale(self, k: Fraction) -> Option<Self> {
        Some(Self::of(tri!(self.value.checked_div(k))))
    }

    /// `self · rhs` in `U · U2`, `None` where the result doesn't fit.
    #[must_use]
    pub const fn checked_times<U2>(self, rhs: Exact<U2>) -> Option<Exact<Times<U, U2>>> {
        Some(Exact::of(tri!(self.value.checked_mul(rhs.value))))
    }

    /// `self / rhs` in `U / U2`, `None` for 0 and where the result doesn't fit.
    #[must_use]
    pub const fn checked_per<U2>(self, rhs: Exact<U2>) -> Option<Exact<Per<U, U2>>> {
        Some(Exact::of(tri!(self.value.checked_div(rhs.value))))
    }

    /// The nearest `f64` (correctly rounded): for plots and logs, never a check.
    #[must_use]
    pub fn to_f64(self) -> f64 {
        self.value.to_f64()
    }
}

/// The factor from `U` to `U2`: same dimension and kind, no offset, both exact, else a compile error.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
const fn factor<U: Unit, U2: Unit>() -> Fraction {
    if !U::DIM.same_typed(U2::DIM) {
        crate::conv_error::dim_mismatch::<U, U2>("an Exact");
    }
    if !U::QUANTITY.converts_typed(U2::QUANTITY) {
        crate::conv_error::kind_mismatch::<U, U2>("an Exact");
    }
    let (Some(from), Some(to)) = (U::EXPR.magnitude(), U2::EXPR.magnitude()) else {
        crate::conv_error::no_exact_factor::<U, U2>("an Exact")
    };
    match Fraction::from_magnitude(&from.div(to)) {
        Some(f) => f,
        None => panic!("the factor between these units is not a fraction of u128s (π, a root)"),
    }
}

/// `1 / (U · U2)`: the factor of a reciprocal from `U` into `U2`, whose dimension is the reciprocal of `U`'s.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
const fn recip_factor<U: Unit, U2: Unit>() -> Fraction {
    if !<Times<U, U2>>::DIM.is_one_typed() {
        crate::conv_error::recip_mismatch::<U, U2>();
    }
    let (Some(a), Some(b)) = (U::EXPR.magnitude(), U2::EXPR.magnitude()) else {
        crate::conv_error::no_exact_factor::<U, U2>("an Exact's recip")
    };
    match Fraction::from_magnitude(&a.mul(b).recip()) {
        Some(f) => f,
        None => panic!("the factor between these units is not a fraction of u128s (π, a root)"),
    }
}

/// The scale `P` as a fraction, a compile error for one with π or a root.
#[expect(
    clippy::panic,
    reason = "a compile error in const code (evaluated in const blocks), as documented"
)]
const fn scale_of<P: FixedScale>() -> Fraction {
    match Fraction::from_magnitude(&P::SCALE) {
        Some(f) => f,
        None => panic!(
            "an Exact goes to and compares with a Fixed of a scale that is a fraction of u128s only (not π, a root, \
             nothing past u128): round through f64"
        ),
    }
}

impl<U: Unit> Exact<U> {
    /// The same value in `U2`, a unit of the same dimension and kind, if it fits: `Times<Hertz, Second>` as a
    /// [Ratio](crate::units::Ratio), minutes as seconds. Units of other dimensions or kinds, with an offset (°C) or a
    /// factor that isn't a fraction (degrees to radians) don't compile.
    #[must_use]
    pub fn convert<U2: Unit>(self) -> Option<Exact<U2>> {
        let k = const { factor::<U, U2>() };
        Some(Exact::of(self.value.checked_mul(k)?))
    }

    /// `1 / self` in `U2`, a unit of the reciprocal dimension: the period of a frequency, `f.recip::<Second>()`. `None`
    /// for 0 and past `u128`; a `U2` of another dimension doesn't compile.
    #[must_use]
    pub fn recip<U2: Unit>(self) -> Option<Exact<U2>> {
        let k = const { recip_factor::<U, U2>() };
        Some(Exact::of(self.value.checked_recip()?.checked_mul(k)?))
    }

    /// The exact value of a [Fixed]: `None` for a float that is NaN or infinite, a scale with π or a root, and past
    /// `u128`.
    #[must_use]
    pub fn from_fixed<P: FixedScale, T: Storage, B: Bounds<T>>(
        f: Fixed<U, P, T, B>,
    ) -> Option<Self> {
        let x = match f.get().to_repr() {
            Repr::Int(i) => Fraction::int(i),
            Repr::Float(x) => Fraction::from_f64(x)?,
        };
        Self::from_scaled(x, &P::SCALE)
    }

    /// The same value as a [Fixed] of scale `P` in an integer storage `T`, if it is a whole number of `P` that fits
    /// `T`, else `None`. A float storage doesn't compile: [to_f64](Self::to_f64) names the rounding.
    #[must_use]
    pub fn to<P: FixedScale, T: Storage>(self) -> Option<Fixed<U, P, T>> {
        let steps = self.steps::<P, T>()?;
        if !steps.is_integer() {
            return None;
        }
        T::from_int(steps.round(Rounding::Truncate)?).and_then(Fixed::from_number)
    }

    /// The same value as a [Fixed] of scale `P` in an integer storage `T`, rounded by `r` where it isn't a whole
    /// number of `P`: `Floor` and `Ceil` are the integers below and above. `None` only when it doesn't fit `T`.
    #[must_use]
    pub fn to_rounded<P: FixedScale, T: Storage>(self, r: Rounding) -> Option<Fixed<U, P, T>> {
        T::from_int(self.steps::<P, T>()?.round(r)?).and_then(Fixed::from_number)
    }

    fn steps<P: FixedScale, T: Storage>(self) -> Option<Fraction> {
        const {
            assert!(
                !T::IS_FLOAT,
                "an Exact goes to a float by name: to_f64, which rounds"
            );
        }
        self.value.checked_div(const { scale_of::<P>() })
    }

    /// This value written in the prefix `p`: `in_prefix(Prefix::Mega)` of 80/3 MHz is `26.(6) MHz`. Where the value
    /// in that prefix doesn't fit `u128`, in the unit itself; a ratio in %, ‰, ppm or ppb, any other prefix as none
    /// (the forms text reads back).
    #[must_use]
    pub const fn in_prefix(self, p: Prefix) -> InPrefix<U> {
        InPrefix { x: self, prefix: p }
    }

    /// The prefix `{}` writes this value with: the largest power of 1000 that leaves at least 1 before the point and
    /// that the unit takes, by its names on a named unit (`eV` all, `km`, `min` and `°` none), on a compound one
    /// where it is a power of ten of SI (none on `m²`); on a ratio %, ppm or ppb, the largest that leaves at least
    /// 0.1, none from 1 up.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "mechanical"))] {
    /// use exact_units::{Exact, Prefix, prefix::Kilo, units::{ElectronVolt, Kilometre}};
    ///
    /// // ⁵⁶Fe's binding energy: eV is 1.602176634·10⁻¹⁹ J, no power of ten of SI, and takes the SI prefixes
    /// let b = Exact::<ElectronVolt>::new_in::<Kilo>(492_259_952_464, 1_000_000);
    /// assert_eq!(b.auto_prefix(), Prefix::Mega);
    /// assert_eq!(b.to_string(), "492.259952464 MeV");
    /// // km is already scaled and takes none: no "kkm"
    /// assert_eq!(Exact::<Kilometre>::int(5000).to_string(), "5000 km");
    /// # }
    /// ```
    #[must_use]
    pub fn auto_prefix(self) -> Prefix {
        let a = self.value.abs();
        if a.is_zero() {
            return Prefix::Unit;
        }
        let at_least = |e: i8| Fraction::pow10(e).is_some_and(|p| a >= p);
        if is_ratio(&U::EXPR) {
            return if at_least(0) {
                Prefix::Unit
            } else if at_least(-3) {
                Prefix::Centi
            } else if at_least(-7) {
                Prefix::Micro
            } else {
                Prefix::Nano
            };
        }
        if !takes_bare_prefix(&U::EXPR) {
            return Prefix::Unit;
        }
        // a named unit takes the prefixes its names allow (eV all of them, though no power of ten of SI; km and min
        // none); a compound one all of them where it is a power of ten of SI
        let decimal = U::EXPR.magnitude().is_some_and(|m| m.as_pow10().is_some());
        let takes = |p: Prefix| match U::EXPR.def() {
            Some(d) => d.names.prefixes.allows(p),
            None => decimal,
        };
        let mut smallest = Prefix::Unit;
        for e in (-10..=10i8).rev().filter_map(|k| k.checked_mul(3)) {
            let Some(p) = Prefix::si_from_pow10(e).filter(|p| takes(*p)) else {
                continue;
            };
            if at_least(e) {
                return p;
            }
            smallest = p;
        }
        smallest
    }
}

/// A quotient `U/U` (Hz/Hz, V/V) is not a plain [Ratio](crate::units::Ratio): these are the named steps between them,
/// as for [Fixed].
#[cfg(feature = "dimensionless")]
impl<U> Exact<Per<U, U>> {
    /// The same number as a plain ratio: what the quotient was a ratio of is gone. Two periods of a 4.8 MHz clock in
    /// cycles of a 199 MHz one:
    ///
    /// ```
    /// # #[cfg(all(feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Exact, prefix::Mega, units::Hertz};
    ///
    /// let hclk = Exact::<Hertz>::new_in::<Mega>(199, 1);
    /// let ref_ck = Exact::<Hertz>::new_in::<Mega>(48, 10);
    /// let cycles = ((hclk * 2).unwrap() / ref_ck).unwrap().as_ratio();
    /// assert_eq!(cycles, Exact::new(995, 12));
    /// # }
    /// ```
    #[must_use]
    pub const fn as_ratio(self) -> Exact<crate::units::Ratio> {
        Exact::of(self.value)
    }
}

#[cfg(feature = "dimensionless")]
impl<U> Exact<U> {
    /// `self · k` in `U` itself: a quantity times a [Ratio](crate::units::Ratio) is the same quantity. `None` where
    /// the result doesn't fit. `self * k` is the same number as an `Exact<Times<U, Ratio>>`, which `convert`s back;
    /// a `Mul` that kept `U` would overlap `Exact<U> * Exact<U2>`, so this is the named step.
    ///
    /// ```
    /// # #[cfg(all(feature = "time", feature = "dimensionless"))] {
    /// use exact_units::{Exact, prefix::Mega, units::{Hertz, Ratio}};
    ///
    /// // an STM32H7 VCO: ref_ck · (N + FRACN/8192)
    /// let ref_ck = Exact::<Hertz>::new_in::<Mega>(8, 1);
    /// let n = Exact::<Ratio>::int(99);
    /// let fracn = Exact::<Ratio>::new(4096, 8192);
    /// let vco = ref_ck.scaled((n + fracn).unwrap()).unwrap();
    /// assert_eq!(vco.to_string(), "796 MHz");
    /// # }
    /// ```
    #[must_use]
    pub const fn scaled(self, k: Exact<crate::units::Ratio>) -> Option<Self> {
        self.checked_scale(k.value)
    }
}

#[cfg(feature = "dimensionless")]
impl Exact<crate::units::Ratio> {
    /// The same number as a quotient `U/U`: `as_gain::<Volt>()` is a voltage gain.
    #[must_use]
    pub const fn as_gain<U>(self) -> Exact<Per<U, U>> {
        Exact::of(self.value)
    }
}

/// An [Exact] written in a prefix: [Exact::in_prefix].
#[derive(Copy, Clone)]
pub struct InPrefix<U> {
    x: Exact<U>,
    prefix: Prefix,
}

impl<U: Unit> fmt::Display for InPrefix<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_in::<U>(self.x.value, self.prefix, f)
    }
}

impl<U: Unit> fmt::Debug for InPrefix<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InPrefix({self})")
    }
}

/// `x` of `U` written in `prefix`, then the unit.
fn write_in<U: Unit>(x: Fraction, prefix: Prefix, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let unit = &U::EXPR;
    // a ratio is read with %, ‰, ppm, ppb or nothing: any other prefix is written as nothing, so it reads back
    let ratio_prefix = matches!(
        prefix,
        Prefix::Unit | Prefix::Centi | Prefix::Milli | Prefix::Micro | Prefix::Nano
    );
    let prefix = if is_ratio(unit) && !ratio_prefix {
        Prefix::Unit
    } else {
        prefix
    };
    // a decimal prefix moves the point; any other scale divides
    let (scaled, shift) = match prefix.pow10() {
        Some(e) => (Some(x), i32::from(e).saturating_neg()),
        None => (
            prefix
                .magnitude()
                .and_then(|m| Fraction::from_magnitude(&m))
                .and_then(|m| x.checked_div(m)),
            0,
        ),
    };
    if let Some(e) = scaled.and_then(|s| Expansion::of(s, shift)) {
        write!(f, "{e}")?;
        return write_unit(prefix, unit, f);
    }
    let in_prefix = match prefix.pow10() {
        Some(e) => Fraction::pow10(e.checked_neg().unwrap_or(0)).and_then(|p| x.checked_mul(p)),
        None => scaled,
    };
    if let Some(s) = in_prefix {
        write_fraction(s, f)?;
        write_unit(prefix, unit, f)
    } else {
        write_fraction(x, f)?;
        write_unit(Prefix::Unit, unit, f)
    }
}

impl<U: Unit> fmt::Display for Exact<U> {
    /// `{}` in the prefix [auto_prefix](Exact::auto_prefix) picks, `{:#}` in the unit itself.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p = if f.alternate() {
            Prefix::Unit
        } else {
            self.auto_prefix()
        };
        write_in::<U>(self.value, p, f)
    }
}

impl<U: Unit> fmt::Debug for Exact<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Exact({:?}", self.value)?;
        write_unit(Prefix::Unit, &U::EXPR, f)?;
        f.write_str(")")
    }
}

impl<U> Clone for Exact<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U> Copy for Exact<U> {}

impl<U> Default for Exact<U> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<U> PartialEq for Exact<U> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<U> Eq for Exact<U> {}

impl<U> PartialOrd for Exact<U> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<U> Ord for Exact<U> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<U> Hash for Exact<U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// Equal where the order is ([PartialOrd]): exactly the same value. For NaN-free values; a [MaybeNan](crate::MaybeNan)
/// compares by [eq_or](Fixed::eq_or).
impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T> + NanFree> PartialEq<Fixed<U, P, T, B>>
    for Exact<U>
{
    fn eq(&self, other: &Fixed<U, P, T, B>) -> bool {
        self.partial_cmp(other) == Some(Ordering::Equal)
    }
}

/// Exactly, across scales, for every value: a float of any size, ±∞ past every `Exact`, always `Some` (NaN only in a [MaybeNan](crate::MaybeNan), which has [lt_or](Fixed::lt_or) and its kin instead). A
/// scale that isn't a fraction of `u128`s (π, a root, past `u128`) doesn't compile, as for [to](Exact::to): an
/// `Exact` is a fraction, and a number of π steps is never one.
///
/// ```
/// # #[cfg(feature = "time")] {
/// use exact_units::{Exact, Fixed, MaybeNan, prefix::{One, Quetta}, units::Hertz};
///
/// let max = Exact::<Hertz>::int(i128::MAX);
/// assert!(Fixed::<Hertz, One, f64>::bounded(1e300) > max);
/// assert!(Fixed::<Hertz, One, f64>::bounded(1e-300) < Exact::new(1, u128::MAX));
/// assert!(Fixed::<Hertz, One, f64>::bounded(f64::NEG_INFINITY) < -max);
/// assert!(Fixed::<Hertz, Quetta, i64>::new(i64::MAX) > max);
/// // NaN only in a MaybeNan, which compares by naming what NaN gives
/// assert!(Fixed::<Hertz, One, f64, MaybeNan>::of(f64::NAN).lt_or(max, true));
/// # }
/// ```
///
/// ```compile_fail
/// # #[cfg(feature = "time")] {
/// use exact_units::{Exact, Fixed, units::Hertz};
///
/// exact_units::scale! { struct PiSteps = "", Exact(1 / 1, pi 1); }
/// let _ = Exact::<Hertz>::int(3) < Fixed::<Hertz, PiSteps, i32>::new(1);
/// # }
/// ```
impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T> + NanFree> PartialOrd<Fixed<U, P, T, B>>
    for Exact<U>
{
    fn partial_cmp(&self, other: &Fixed<U, P, T, B>) -> Option<Ordering> {
        compare::order(other.get().to_repr(), const { scale_of::<P>() }, self.value)
            .map(Ordering::reverse)
    }
}

/// See `PartialEq<Fixed> for Exact`.
impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T> + NanFree> PartialEq<Exact<U>>
    for Fixed<U, P, T, B>
{
    fn eq(&self, other: &Exact<U>) -> bool {
        other == self
    }
}

/// See `PartialOrd<Fixed> for Exact`.
impl<U: Unit, P: FixedScale, T: Storage, B: Bounds<T> + NanFree> PartialOrd<Exact<U>>
    for Fixed<U, P, T, B>
{
    fn partial_cmp(&self, other: &Exact<U>) -> Option<Ordering> {
        compare::order(self.get().to_repr(), const { scale_of::<P>() }, other.value)
    }
}

impl<U> Add for Exact<U> {
    type Output = Option<Self>;

    /// [checked_add](Exact::checked_add).
    fn add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
}

impl<U> Sub for Exact<U> {
    type Output = Option<Self>;

    /// [checked_sub](Exact::checked_sub).
    fn sub(self, rhs: Self) -> Option<Self> {
        self.checked_sub(rhs)
    }
}

impl<U> Neg for Exact<U> {
    type Output = Self;

    fn neg(self) -> Self {
        Self::of(self.value.negate())
    }
}

impl<U> Mul<i128> for Exact<U> {
    type Output = Option<Self>;

    fn mul(self, n: i128) -> Option<Self> {
        self.checked_scale(Fraction::int(n))
    }
}

impl<U> Div<i128> for Exact<U> {
    type Output = Option<Self>;

    fn div(self, n: i128) -> Option<Self> {
        self.checked_unscale(Fraction::int(n))
    }
}

impl<U> Mul<Fraction> for Exact<U> {
    type Output = Option<Self>;

    fn mul(self, k: Fraction) -> Option<Self> {
        self.checked_scale(k)
    }
}

impl<U> Div<Fraction> for Exact<U> {
    type Output = Option<Self>;

    fn div(self, k: Fraction) -> Option<Self> {
        self.checked_unscale(k)
    }
}

impl<U, U2> Mul<Exact<U2>> for Exact<U> {
    type Output = Option<Exact<Times<U, U2>>>;

    /// [checked_times](Exact::checked_times).
    fn mul(self, rhs: Exact<U2>) -> Self::Output {
        self.checked_times(rhs)
    }
}

impl<U, U2> Div<Exact<U2>> for Exact<U> {
    type Output = Option<Exact<Per<U, U2>>>;

    /// [checked_per](Exact::checked_per).
    fn div(self, rhs: Exact<U2>) -> Self::Output {
        self.checked_per(rhs)
    }
}

#[cfg(test)]
mod tests {
    /// [bit_length_halved] is `bit_width` for every bit length of a `u128`, a `u16` and a `u8`, at
    /// both ends of each (`2^p` and `2^(p+1) − 1`): its branches look only at whether `x >> width` is 0, which
    /// depends only on the highest bit set, so the two ends of each length take every path it can.
    #[test]
    fn bit_length_halved_is_bit_width() {
        macro_rules! walk {
            ($u:ty) => {
                assert_eq!(bit_length_halved!(0, $u), 0);
                for p in 0..<$u>::BITS {
                    let lo: $u = 1 << p;
                    let hi: $u = lo.wrapping_sub(1).wrapping_add(lo);
                    for x in [lo, hi] {
                        assert_eq!(bit_length_halved!(x, $u), x.bit_width(), "{x}");
                        assert_eq!(bit_length!(x, $u), x.bit_width(), "{x}");
                    }
                }
            };
        }
        walk!(u128);
        walk!(u16);
        walk!(u8);
    }
}
