//! [Undefined]: why the power factor of a [Complex](super::Complex) apparent power has no value, named for the
//! physics. A storage that can't hold a value that exists is `None` elsewhere in the crate, and a point at infinity
//! is an [Extended](super::Extended) value (Z, Y, Γ); these are the inputs where no value exists to hold.

use core::fmt;

/// Why [power_factor](super::Complex::power_factor) has no value: P / |S| is 0 / 0 or ∞ / ∞, which no point of the
/// plane, finite or not, is. Its `Display` says what happened, for a log line.
///
/// ```
/// # #[cfg(all(feature = "electrical", feature = "dimensionless"))] {
/// use exact_units::{Complex, Undefined, prefix::One, units::VoltAmpere};
///
/// let idle = Complex::<VoltAmpere, One, i32>::new(0, 0);
/// assert_eq!(idle.power_factor(), Err(Undefined::NoApparentPower));
/// assert_eq!(
///     Undefined::NoApparentPower.to_string(),
///     "no apparent power (S = 0): the power factor is 0 / 0"
/// );
/// # }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Undefined {
    /// [power_factor](super::Complex::power_factor) of S = 0: no apparent power, so P / |S| is 0 / 0 and no load
    /// angle is seen.
    NoApparentPower,
    /// [power_factor](super::Complex::power_factor) of an S whose active and reactive parts are both infinite: the
    /// angle between them is ∞ / ∞. One infinite part alone gives λ = ±1 (active) or 0 (reactive).
    InfinitePower,
}

impl fmt::Display for Undefined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Undefined::NoApparentPower => "no apparent power (S = 0): the power factor is 0 / 0",
            Undefined::InfinitePower => {
                "active and reactive power are both infinite: the power factor is ∞ / ∞"
            }
        })
    }
}

impl core::error::Error for Undefined {}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn display_says_what_happened() {
        for (cause, text) in [
            (
                Undefined::NoApparentPower,
                "no apparent power (S = 0): the power factor is 0 / 0",
            ),
            (
                Undefined::InfinitePower,
                "active and reactive power are both infinite: the power factor is ∞ / ∞",
            ),
        ] {
            assert_eq!(cause.to_string(), text);
        }
    }
}
