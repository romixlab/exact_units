//! [Extended]: a value on the extended complex plane, the complex numbers and one point ∞ (the Riemann sphere), as a
//! Smith chart draws them. An impedance of an ideal short has an admittance there, and an open an impedance.

use core::fmt;

/// A value or the point at infinity: the extended complex plane of an [Impedance](crate::Impedance), an
/// [Admittance](crate::Admittance) or a reflection coefficient Γ. A short's admittance is [Infinity](Self::Infinity),
/// as is an open's impedance and the Γ of a load Z = −Z₀, so 1 / Z and Γ have a value for every input, ∞ included.
///
/// ∞ has no direction: one point, whatever the parts that went there. Its names where the readers look are
/// [Impedance::OPEN](crate::Complex::OPEN) and [Admittance::SHORT](crate::Complex::SHORT), and `is_open` /
/// `is_short` on an impedance or admittance, plain or `Extended`.
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Admittance, Extended, Impedance, prefix::One};
///
/// let short = Impedance::<One, f64>::bounded(0.0, 0.0);
/// let y: Extended<Admittance<One, f64>> = short.to_admittance();
/// assert_eq!(y, Admittance::SHORT);
/// assert!(y.is_short() && y.is_infinite());
/// assert_eq!(y.finite(), None);
/// assert_eq!(y.to_string(), "∞");
/// match y {
///     Extended::Finite(y) => println!("{y}"),
///     Extended::Infinity => println!("a short"),
/// }
/// # }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Extended<T> {
    /// A value of the plane.
    Finite(T),
    /// The one point at infinity.
    Infinity,
}

impl<T> Extended<T> {
    /// The value, `None` for [Infinity](Self::Infinity).
    #[inline(always)]
    pub fn finite(self) -> Option<T> {
        match self {
            Extended::Finite(value) => Some(value),
            Extended::Infinity => None,
        }
    }

    /// Whether it is the point at infinity.
    #[inline(always)]
    pub const fn is_infinite(&self) -> bool {
        matches!(self, Extended::Infinity)
    }

    /// Whether it is a value of the plane.
    #[inline(always)]
    pub const fn is_finite(&self) -> bool {
        matches!(self, Extended::Finite(_))
    }

    /// `f` of the value, ∞ for ∞.
    #[inline(always)]
    pub fn map<T2>(self, f: impl FnOnce(T) -> T2) -> Extended<T2> {
        match self {
            Extended::Finite(value) => Extended::Finite(f(value)),
            Extended::Infinity => Extended::Infinity,
        }
    }
}

impl<T> From<T> for Extended<T> {
    /// The value, [Finite](Extended::Finite).
    #[inline(always)]
    fn from(value: T) -> Self {
        Extended::Finite(value)
    }
}

/// The value as `T` shows it, with the formatter's flags; `∞` padded to the width.
impl<T: fmt::Display> fmt::Display for Extended<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Extended::Finite(value) => value.fmt(f),
            Extended::Infinity => f.pad("∞"),
        }
    }
}

/// The value as `T` shows it, `∞` for the point.
#[cfg(feature = "defmt")]
impl<T: defmt::Format> defmt::Format for Extended<T> {
    fn format(&self, f: defmt::Formatter<'_>) {
        match self {
            Extended::Finite(value) => value.format(f),
            Extended::Infinity => defmt::write!(f, "∞"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors() {
        let one = Extended::from(1u8);
        assert_eq!(one, Extended::Finite(1));
        assert_eq!(one.finite(), Some(1));
        assert!(one.is_finite() && !one.is_infinite());
        assert_eq!(one.map(|x| x + 1), Extended::Finite(2));
        let inf = Extended::<u8>::Infinity;
        assert_eq!(inf.finite(), None);
        assert!(inf.is_infinite() && !inf.is_finite());
        assert_eq!(inf.map(|x| x + 1), Extended::Infinity);
        assert_ne!(one, inf);
    }

    #[test]
    #[cfg(feature = "std")]
    fn display() {
        assert_eq!(Extended::Finite(1.5f64).to_string(), "1.5");
        assert_eq!(format!("{:.2}", Extended::Finite(1.0f64)), "1.00");
        assert_eq!(Extended::<f64>::Infinity.to_string(), "∞");
        assert_eq!(format!("{:>3}", Extended::<f64>::Infinity), "  ∞");
    }
}
