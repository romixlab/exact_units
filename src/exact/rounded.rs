//! [Exact::rounded], [Exact::rounded_at], [Exact::rounded_to]: a decimal for a person, rounded by a named mode to a
//! number of places or to a power of ten of the unit.

use super::num::sealed::Sealed as _;
use super::{Exact, ExactNum, Fraction, prefix_for, value_in, write_in};
use crate::tree::write_unit;
use crate::{MagnitudeOf, Prefix, Rounding, Unit};
use core::fmt;

impl<U: Unit> Exact<U> {
    /// This value as a decimal with `places` digits after the point, rounded by `r`: for a person, where the exact
    /// form is a long repetend or a fraction (⁶²Ni's binding energy per nucleon is `68157805479/7750000 keV`). The
    /// rounding is named here, never by a format's precision, and the text it writes reads back as the rounded value,
    /// not this one.
    ///
    /// `{}` writes it in the prefix `{}` of the value picks, or the one [in_prefix](Rounded::in_prefix) names,
    /// `{:#}` in the unit itself, `{:e}` and `{:E}` in scientific notation in the unit itself with `places` digits after the mantissa's point. A value whose
    /// rounded digits don't fit an `i128` (more than 38) is written exactly, as `{}` writes it.
    ///
    /// ```
    /// # #[cfg(all(feature = "radiation", feature = "mechanical"))] {
    /// use exact_units::{Exact, Prefix, Rounding, prefix::Kilo, units::{ElectronVolt, Meter}};
    ///
    /// let b_per_a = Exact::<ElectronVolt>::new_in::<Kilo>(68_157_805_479, 7_750_000);
    /// assert_eq!(b_per_a.in_prefix(Prefix::Kilo).to_string(), "68157805479/7750000 keV");
    /// let r = b_per_a.rounded(4, Rounding::NearestEven);
    /// assert_eq!(r.to_string(), "8.7946 MeV");
    /// assert_eq!(r.in_prefix(Prefix::Kilo).to_string(), "8794.5555 keV");
    /// assert_eq!(b_per_a.rounded(1, Rounding::Floor).in_prefix(Prefix::Kilo).to_string(), "8794.5 keV");
    /// assert_eq!(format!("{:#}", b_per_a.rounded(0, Rounding::Ceil)), "8794556 eV");
    /// assert_eq!(format!("{:e}", b_per_a.rounded(3, Rounding::NearestEven)), "8.795e6 eV");
    /// // places count after the point, trailing zeros kept: the digits say how far it was rounded
    /// let l = Exact::<Meter>::new(-1, 8);
    /// assert_eq!(l.rounded(4, Rounding::NearestEven).to_string(), "-125.0000 mm");
    /// assert_eq!(format!("{:E}", l.rounded(2, Rounding::Truncate)), "-1.25E-1 m");
    /// # }
    /// ```
    #[must_use]
    pub const fn rounded(self, places: u8, r: Rounding) -> Rounded<U> {
        Rounded::of(self, places, r)
    }

    /// This value rounded by `r` to a whole number of `10^decade` of `U`, and written with the digits down to that
    /// step: a ruler's label at a zoom level, a timestamp to its clock's resolution. The rounding is in `U`, so it is
    /// the same whatever prefix writes it; `{}` picks the prefix as for [rounded](Self::rounded), and a step above
    /// the prefix's own writes the zeros it rounded away.
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Exact, Prefix, Rounding, units::Second};
    ///
    /// let t = Exact::<Second>::new(12_345_678, 1_000_000_000);
    /// // a ruler at 1 µs, at 0.1 ms and at 10 ms
    /// assert_eq!(t.rounded_at(-6, Rounding::NearestEven).to_string(), "12.346 ms");
    /// assert_eq!(t.rounded_at(-4, Rounding::Floor).to_string(), "12.3 ms");
    /// assert_eq!(t.rounded_at(-2, Rounding::NearestEven).to_string(), "10 ms");
    /// assert_eq!(format!("{:#}", t.rounded_at(-6, Rounding::Floor)), "0.012345 s");
    /// assert_eq!(t.rounded_at(-6, Rounding::Floor).in_prefix(Prefix::Micro).to_string(), "12345 µs");
    /// assert_eq!(format!("{:e}", t.rounded_at(-5, Rounding::Ceil)), "1.235e-2 s");
    /// # }
    /// ```
    #[must_use]
    pub const fn rounded_at(self, decade: i32, r: Rounding) -> Rounded<U> {
        Rounded {
            x: self,
            digits: Digits::Decade(decade),
            rounding: r,
            prefix: None,
        }
    }

    /// This value to the decimal digits a `resolution` resolves, rounded by `r`: [rounded_at](Self::rounded_at) at
    /// the decade `k` with `10^k <= resolution < 10^(k+1)`, so a clock of 1 µs shows µs digits and one of 1/32768 s
    /// (30.5 µs) shows 10 µs digits, never digits finer than the clock. `None` for a resolution without an exact
    /// decade here (π, a root, past `u128`).
    ///
    /// ```
    /// # #[cfg(feature = "time")] {
    /// use exact_units::{Exact, Magnitude, MagnitudeOf, Rounding, units::Second};
    ///
    /// let t = Exact::<Second>::new(12_345_678, 1_000_000_000);
    /// let crystal = MagnitudeOf::<Second>::new(Magnitude::ratio(1, 32_768));
    /// assert_eq!(t.rounded_to(crystal, Rounding::NearestEven).unwrap().to_string(), "12.35 ms");
    /// let us = MagnitudeOf::<Second>::new(Magnitude::pow10(-6));
    /// assert_eq!(t.rounded_to(us, Rounding::Floor).unwrap().to_string(), "12.345 ms");
    /// # }
    /// ```
    #[must_use]
    pub fn rounded_to(self, resolution: MagnitudeOf<U>, r: Rounding) -> Option<Rounded<U>> {
        let k = resolution.to_exact()?.value.decade()?;
        Some(self.rounded_at(k, r))
    }
}

impl<U, R: ExactNum> Rounded<U, R> {
    pub(crate) const fn of(x: Exact<U, R>, places: u8, r: Rounding) -> Self {
        Rounded {
            x,
            digits: Digits::Places(places),
            rounding: r,
            prefix: None,
        }
    }
}

/// How far a [Rounded] rounds.
#[derive(Copy, Clone, Debug)]
enum Digits {
    /// To this many places after the point of the prefix it is written in: [Exact::rounded].
    Places(u8),
    /// To a whole number of `10^k` of the unit: [Exact::rounded_at].
    Decade(i32),
}

/// An [Exact] as a decimal rounded by a named mode: [Exact::rounded], [Exact::rounded_at], [Exact::rounded_to].
pub struct Rounded<U, R: ExactNum = Fraction> {
    x: Exact<U, R>,
    digits: Digits,
    rounding: Rounding,
    prefix: Option<Prefix>,
}

impl<U> Rounded<U> {
    /// The same, written in the prefix `p` by `{}`: a binding energy per nucleon in keV. A ratio takes %, ‰, ppm,
    /// ppb or none, as [Exact::in_prefix].
    #[must_use]
    pub const fn in_prefix(self, p: Prefix) -> Self {
        Self {
            prefix: Some(p),
            ..self
        }
    }
}

#[cfg(feature = "bignum")]
impl<U> Rounded<U, super::BigFraction> {
    /// The same, written in the prefix `p` by `{}`, as [Rounded::in_prefix] of a [Fraction].
    #[must_use]
    pub fn in_prefix(mut self, p: Prefix) -> Self {
        self.prefix = Some(p);
        self
    }
}

impl<U, R: ExactNum> Clone for Rounded<U, R> {
    fn clone(&self) -> Self {
        Self {
            x: self.x.clone(),
            ..*self
        }
    }
}

impl<U, R: ExactNum + Copy> Copy for Rounded<U, R> {}

impl<U: Unit, R: ExactNum> fmt::Debug for Rounded<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rounded({self}, {:?})", self.rounding)
    }
}

/// `x · 10^places` rounded by `r`, `None` past `i128`.
fn steps<R: ExactNum>(x: &R, places: u8, r: Rounding) -> Option<i128> {
    x.checked_mul(&R::from_fraction(Fraction::pow10(places.into())))?
        .round(r)
}

/// `x · 10^e`, `None` where the result doesn't fit.
fn times_pow10<R: ExactNum>(x: &R, e: i32) -> Option<R> {
    x.checked_mul(&R::from_fraction(Fraction::pow10(e)))
}

/// `n / 10^places` with exactly `places` digits after the point; the sign of `n`.
fn write_places(n: i128, places: u8, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let sign = if n < 0 { "-" } else { "" };
    let a = n.unsigned_abs();
    if places == 0 {
        return write!(f, "{sign}{a}");
    }
    let p = 10u128.checked_pow(u32::from(places)).ok_or(fmt::Error)?;
    let (whole, frac) = (
        a.checked_div(p).ok_or(fmt::Error)?,
        a.checked_rem(p).ok_or(fmt::Error)?,
    );
    let width = usize::from(places);
    write!(f, "{sign}{whole}.{frac:0width$}")
}

/// `x` rounded by `r` to a whole number of `10^k`: the number of steps and the rounded value, `None` past `i128`.
fn at_decade<R: ExactNum>(x: &R, k: i32, r: Rounding) -> Option<(i128, R)> {
    let n = times_pow10(x, k.checked_neg()?)?.round(r)?;
    let v = Fraction::int(n).checked_mul(Fraction::pow10(k))?;
    Some((n, R::from_fraction(v)))
}

impl<U: Unit, R: ExactNum> fmt::Display for Rounded<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if f.alternate() {
            Prefix::Unit
        } else {
            self.prefix
                .unwrap_or_else(|| super::auto_prefix_of::<U, R>(&self.x.value))
        };
        let prefix = prefix_for::<U>(prefix);
        match self.digits {
            Digits::Places(places) => {
                let in_prefix = value_in(&self.x.value, prefix);
                match in_prefix.and_then(|v| steps(&v, places, self.rounding)) {
                    Some(n) => {
                        write_places(n, places, f)?;
                        write_unit(prefix, &U::EXPR, f)
                    }
                    None => write_in::<U, R>(&self.x.value, prefix, f),
                }
            }
            Digits::Decade(k) => self.write_decade(k, prefix, f),
        }
    }
}

impl<U: Unit, R: ExactNum> Rounded<U, R> {
    /// The value rounded to a whole number of `10^k` of `U`, written in `prefix` with the digits down to that step;
    /// written exactly where the digits don't fit an `i128` or the prefix isn't a power of ten.
    fn write_decade(&self, k: i32, prefix: Prefix, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((n, v)) = at_decade(&self.x.value, k, self.rounding) else {
            return write_in::<U, R>(&self.x.value, prefix, f);
        };
        // the digits after the prefix's point, or below 0 the zeros before it
        let places = prefix.pow10().and_then(|e| i32::from(e).checked_sub(k));
        if let Some(p) = places
            .and_then(|p| u8::try_from(p).ok())
            .filter(|p| *p <= 38)
        {
            write_places(n, p, f)?;
            return write_unit(prefix, &U::EXPR, f);
        }
        let zeros = places
            .and_then(i32::checked_neg)
            .and_then(|z| u32::try_from(z).ok())
            .and_then(|z| 10i128.checked_pow(z))
            .and_then(|t| n.checked_mul(t));
        match zeros {
            Some(m) => {
                write_places(m, 0, f)?;
                write_unit(prefix, &U::EXPR, f)
            }
            None => write_in::<U, R>(&v, prefix, f),
        }
    }

    /// Scientific notation in the unit itself, `e` the letter of the exponent.
    fn write_exp(&self, e: char, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.digits {
            Digits::Places(places) => {
                write_exp_of::<U, R>(&self.x.value, places, self.rounding, e, f)
            }
            Digits::Decade(k) => {
                let Some((_, v)) = at_decade(&self.x.value, k, self.rounding) else {
                    return write_in::<U, R>(&self.x.value, Prefix::Unit, f);
                };
                // the mantissa's digits down to 10^k: v is whole in them, so no second rounding
                let places = match v.decade() {
                    Some(kv) => kv.checked_sub(k).and_then(|p| u8::try_from(p).ok()),
                    None => Some(0),
                };
                match places {
                    Some(p) => write_exp_of::<U, R>(&v, p, Rounding::Truncate, e, f),
                    None => write_in::<U, R>(&v, Prefix::Unit, f),
                }
            }
        }
    }
}

/// `x` of `U` in scientific notation in the unit itself with `places` digits after the mantissa's point, rounded by
/// `r`, `e` the letter of the exponent.
fn write_exp_of<U: Unit, R: ExactNum>(
    x: &R,
    places: u8,
    r: Rounding,
    e: char,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if x.is_zero() {
        write_places(0, places, f)?;
        write!(f, "{e}0")?;
        return write_unit(Prefix::Unit, &U::EXPR, f);
    }
    // k with 10^k <= |x| < 10^(k+1)
    let Some(mut k) = x.decade() else {
        return write_in::<U, R>(x, Prefix::Unit, f);
    };
    let mantissa = |k: i32| steps(&times_pow10(x, k.checked_neg()?)?, places, r);
    let Some(mut n) = mantissa(k) else {
        return write_in::<U, R>(x, Prefix::Unit, f);
    };
    // rounding up to 10.0..0: one more power of ten
    let ten = 10i128.checked_pow(u32::from(places).saturating_add(1));
    if ten.is_some_and(|t| n.unsigned_abs() >= t.unsigned_abs()) {
        k = k.saturating_add(1);
        n = n.checked_div(10).ok_or(fmt::Error)?;
    }
    write_places(n, places, f)?;
    write!(f, "{e}{k}")?;
    write_unit(Prefix::Unit, &U::EXPR, f)
}

impl<U: Unit, R: ExactNum> fmt::LowerExp for Rounded<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_exp('e', f)
    }
}

impl<U: Unit, R: ExactNum> fmt::UpperExp for Rounded<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_exp('E', f)
    }
}
