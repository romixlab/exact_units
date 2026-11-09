//! This crate's quantities: parts of ISO 80000's tree, the quantity markers of [Of<Q, U>](crate::units::Of)
//! ([quantity](mod@crate::quantity) has the rules). A unit measures its dimension's quantity unless its line names one,
//! so most units need none of these: `Meter` is a [Length], `Joule` an [Energy].
//!
//! - Length (ISO 80000-3): [Length] → [Height], [Width], [Radius], [Distance] → [PathLength].
//! - Energy (ISO 80000-4, -5): [Energy] → [MechanicalEnergy] → [KineticEnergy], [PotentialEnergy]; [Energy] →
//!   [Work], [Heat].
//! - s⁻¹: [Frequency] (`Hertz`), and two kinds of their own, [Activity] (`Becquerel`) and [SymbolRate] (`Baud`):
//!   Hz, Bq and Bd don't convert.
//! - m²·s⁻²: [AbsorbedDose] (`Gray`) and [DoseEquivalent] (`Sievert`), kinds of their own, apart by a weighting
//!   factor.
//! - Electrical: [Resistance] (`Ohm`) and [Conductance] (`Siemens`), apart from V/A and A/V; [Power] with
//!   [ApparentPower] (`VoltAmpere`) and [ReactivePower] (`Var`).
//! - [Pressure] and [EnergyDensity] (one dimension, two quantities); [Entropy], apart from J/K's own quantity.
//! - Dimension one: [DampingRatio] and [QualityFactor].
//! - Angles: [PlaneAngle] and [PhaseAngle], the angle of a phasor or of a motor's electrical rotation.
//!
//! [ALL] lists them, the table [units::TABLE](crate::units::TABLE) resolves them by id.

pub use crate::quantity::Common;

crate::quantity! {
    @table
    /// Every quantity of this crate.
    pub static ALL;

    /// Length, l (ISO 80000-3, 3-1.1): the open root of m.
    pub struct Length = [m 1];
    /// Height, h: a length measured upwards.
    pub struct Height: Length;
    /// Width, b: a length across.
    pub struct Width: Length;
    /// Radius, r: a length from a centre.
    pub struct Radius: Length;
    /// Distance, d: the length between two points.
    pub struct Distance: Length;
    /// Path length, s: a distance along a path.
    pub struct PathLength: Distance;

    /// Energy, E (ISO 80000-5): the open root of J.
    pub struct Energy = [kg 1, m 2, s -2];
    /// Mechanical energy: kinetic and potential.
    pub struct MechanicalEnergy: Energy;
    /// Kinetic energy, ½·m·v².
    pub struct KineticEnergy: MechanicalEnergy;
    /// Potential energy, m·g·h.
    pub struct PotentialEnergy: MechanicalEnergy;
    /// Work, W: energy moved by a force along a path.
    pub struct Work: Energy;
    /// Heat, Q: energy moved by a difference of temperature.
    pub struct Heat: Energy;

    /// Frequency, f (ISO 80000-3): the open root of s⁻¹, [Hertz](crate::units::Hertz)'s.
    pub struct Frequency = [s -1];
    /// Activity, A (ISO 80000-10): decays per second, [Becquerel](crate::units::Becquerel)'s; not a frequency.
    pub struct Activity = kind [s -1];
    /// Symbol rate: symbols per second, [Baud](crate::units::Baud)'s; not a clock's frequency.
    pub struct SymbolRate = kind [s -1];

    /// Absorbed dose, D (ISO 80000-10), [Gray](crate::units::Gray)'s.
    pub struct AbsorbedDose = kind [m 2, s -2];
    /// Dose equivalent, H: an absorbed dose weighted by its radiation, [Sievert](crate::units::Sievert)'s.
    pub struct DoseEquivalent = kind [m 2, s -2];

    /// Resistance, R (ISO 80000-6): a component's V/A, not an amplifier's transimpedance.
    pub struct Resistance = kind [kg 1, m 2, s -3, a -2];
    /// Conductance, G: a component's A/V, not an amplifier's transconductance.
    pub struct Conductance = kind [kg -1, m -2, s 3, a 2];
    /// Power, P: the open root of W.
    pub struct Power = [kg 1, m 2, s -3];
    /// Apparent power, |S|: the magnitude of the complex power, [VoltAmpere](crate::units::VoltAmpere)'s.
    pub struct ApparentPower = kind [kg 1, m 2, s -3];
    /// Reactive power, Q: the imaginary part of the complex power, [Var](crate::units::Var)'s.
    pub struct ReactivePower = kind [kg 1, m 2, s -3];

    /// Pressure, p: the open root of Pa.
    pub struct Pressure = [kg 1, m -1, s -2];
    /// Energy density, w: energy per volume, not a pressure.
    pub struct EnergyDensity = kind [kg 1, m -1, s -2];
    /// Entropy, S: not a heat capacity, though both are J/K ([HeatCapacity](crate::units::HeatCapacity) measures
    /// the dimension's own quantity, a heat capacity).
    pub struct Entropy = kind [kg 1, m 2, s -2, k -1];

    /// Damping ratio, ζ, of a second-order system: a number of its own, not a plain ratio.
    pub struct DampingRatio = kind [];
    /// Quality factor, Q = 1/(2ζ): a number of its own.
    pub struct QualityFactor = kind [];

    /// Plane angle, the open root of rad.
    pub struct PlaneAngle = [rad 1];
    /// Phase angle, φ: the argument of a phasor, an NCO's phase, a motor's electrical angle. Not a mechanical
    /// angle: the named steps `as_phase` and `as_plane_angle` cross.
    pub struct PhaseAngle: kind PlaneAngle;
}
