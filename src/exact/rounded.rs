//! [Exact::rounded]: a decimal for a person, rounded by a named mode to a number of places.

use super::{Exact, ExactNum, Fraction, prefix_for, value_in, write_in};
use crate::tree::write_unit;
use crate::{Prefix, Rounding, Unit};
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
}

impl<U, R: ExactNum> Rounded<U, R> {
    pub(crate) const fn of(x: Exact<U, R>, places: u8, r: Rounding) -> Self {
        Rounded {
            x,
            places,
            rounding: r,
            prefix: None,
        }
    }
}

/// An [Exact] as a decimal rounded by a named mode: [Exact::rounded].
pub struct Rounded<U, R: ExactNum = Fraction> {
    x: Exact<U, R>,
    places: u8,
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

impl<U: Unit, R: ExactNum> fmt::Display for Rounded<U, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if f.alternate() {
            Prefix::Unit
        } else {
            self.prefix
                .unwrap_or_else(|| super::auto_prefix_of::<U, R>(&self.x.value))
        };
        let prefix = prefix_for::<U>(prefix);
        let in_prefix = value_in(&self.x.value, prefix);
        match in_prefix.and_then(|v| steps(&v, self.places, self.rounding)) {
            Some(n) => {
                write_places(n, self.places, f)?;
                write_unit(prefix, &U::EXPR, f)
            }
            None => write_in::<U, R>(&self.x.value, prefix, f),
        }
    }
}

impl<U: Unit, R: ExactNum> Rounded<U, R> {
    /// Scientific notation in the unit itself, `e` the letter of the exponent.
    fn write_exp(&self, e: char, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = &self.x.value;
        if x.is_zero() {
            write_places(0, self.places, f)?;
            write!(f, "{e}0")?;
            return write_unit(Prefix::Unit, &U::EXPR, f);
        }
        // k with 10^k <= |x| < 10^(k+1)
        let Some(mut k) = x.decade() else {
            return write_in::<U, R>(x, Prefix::Unit, f);
        };
        let mantissa = |k: i32| {
            steps(
                &times_pow10(x, k.checked_neg()?)?,
                self.places,
                self.rounding,
            )
        };
        let Some(mut n) = mantissa(k) else {
            return write_in::<U, R>(x, Prefix::Unit, f);
        };
        // rounding up to 10.0..0: one more power of ten
        let ten = 10i128.checked_pow(u32::from(self.places).saturating_add(1));
        if ten.is_some_and(|t| n.unsigned_abs() >= t.unsigned_abs()) {
            k = k.saturating_add(1);
            n = n.checked_div(10).ok_or(fmt::Error)?;
        }
        write_places(n, self.places, f)?;
        write!(f, "{e}{k}")?;
        write_unit(Prefix::Unit, &U::EXPR, f)
    }
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
