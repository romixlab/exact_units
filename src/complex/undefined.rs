//! [Undefined]: why an RF or power quantity of a [Complex](super::Complex) has no value at an input, named for the
//! physics. A storage that can't hold a value that exists is `None` elsewhere in the crate; these are the inputs
//! where no value exists to hold.

use core::fmt;

/// Why [reflection](super::Complex::reflection), [to_admittance](super::Complex::to_admittance),
/// [to_impedance](super::Complex::to_impedance) or [power_factor](super::Complex::power_factor) has no value: the
/// input is one where the quantity is 0 / 0 or ∞ / ∞, not one whose result is too large for the storage. Its
/// `Display` says what happened, for a log line.
///
/// ```
/// # #[cfg(feature = "electrical")] {
/// use exact_units::{Impedance, Undefined, prefix::One};
///
/// let short = Impedance::<One, f64>::bounded(0.0, 0.0);
/// assert_eq!(short.to_admittance::<One>(), Err(Undefined::Short));
/// assert_eq!(Undefined::Short.to_string(), "a short (Z = 0) has no admittance");
/// # }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Undefined {
    /// [reflection](super::Complex::reflection) of a load Z = −Z₀: Γ = (Z − Z₀) / (Z + Z₀) is a nonzero number
    /// over 0, or 0 / 0 for Z = Z₀ = 0. A passive load on a passive line never meets it; a negative resistance
    /// (an oscillator's, a tunnel diode's) can.
    LoadIsMinusZ0,
    /// [reflection](super::Complex::reflection) with Z and Z₀ both infinite: ∞ / ∞. An infinite Z on a finite Z₀
    /// is an open (Γ = 1), a finite Z on an infinite Z₀ is Γ = −1.
    BothInfinite,
    /// [to_admittance](super::Complex::to_admittance) of Z = 0: an ideal short conducts without limit, and Y = 1 / 0
    /// has no direction to be infinite in.
    Short,
    /// [to_impedance](super::Complex::to_impedance) of Y = 0: an ideal open, Z = 1 / 0.
    Open,
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
            Undefined::LoadIsMinusZ0 => {
                "the load is minus the reference impedance (Z = −Z₀): Γ has a zero denominator"
            }
            Undefined::BothInfinite => {
                "the load and the reference impedance are both infinite: Γ is ∞ / ∞"
            }
            Undefined::Short => "a short (Z = 0) has no admittance",
            Undefined::Open => "an open (Y = 0) has no impedance",
            Undefined::NoApparentPower => "no apparent power (S = 0): the power factor is 0 / 0",
            Undefined::InfinitePower => {
                "active and reactive power are both infinite: the power factor is ∞ / ∞"
            }
        })
    }
}

impl core::error::Error for Undefined {}
