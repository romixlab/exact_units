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
//! - Electromagnetic, the SI side of Gaussian's rows ([systems](crate::systems)): [ElectricCurrent],
//!   [ElectricCharge], [Voltage], [ElectricField], [MagneticFlux], [MagneticFluxDensity], [Capacitance],
//!   [Inductance] as open roots; at C/m² and A/m, whose quantities Gaussian counts 4π apart, no open root:
//!   [ElectricFluxDensity] (D) with [ElectricPolarization] under it, [SurfaceChargeDensity]; [MagnetizingField] (H)
//!   with [Magnetization] under it, [LinearCurrentDensity]. [MagnetomotiveForce] apart from a current.
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

    /// Electric current, I (ISO 80000-6): the open root of A.
    pub struct ElectricCurrent = [a 1];
    /// Magnetomotive force, F_m: ampere-turns around a magnetic circuit, not a current (Gaussian's gilbert is
    /// another unit than its statampere).
    pub struct MagnetomotiveForce = kind [a 1];
    /// Electric charge, Q: the open root of C.
    pub struct ElectricCharge = [s 1, a 1];
    /// Voltage, U: a potential difference or an electromotive force, the open root of V.
    pub struct Voltage = [kg 1, m 2, s -3, a -1];
    /// Electric field strength, E: the open root of V/m.
    pub struct ElectricField = [kg 1, m 1, s -3, a -1];
    /// Electric flux density, D (the displacement field): a kind of its own, as C/m² is also a polarization and a
    /// surface charge density, which Gaussian units count 4π apart.
    pub struct ElectricFluxDensity = kind [s 1, a 1, m -2];
    /// Electric polarization, P: a kind of its own under D (D = ε₀E + P, after `as_quantity`).
    pub struct ElectricPolarization: kind ElectricFluxDensity;
    /// Surface density of charge, σ: a kind of its own, not a flux density.
    pub struct SurfaceChargeDensity = kind [s 1, a 1, m -2];
    /// Magnetic flux, Φ: the open root of Wb.
    pub struct MagneticFlux = [kg 1, m 2, s -2, a -1];
    /// Magnetic flux density, B: the open root of T.
    pub struct MagneticFluxDensity = [kg 1, s -2, a -1];
    /// Magnetic field strength, H (the magnetizing field): a kind of its own, as A/m is also a magnetization and a
    /// linear current density, which Gaussian units count 4π apart.
    pub struct MagnetizingField = kind [a 1, m -1];
    /// Magnetization, M: a kind of its own under H (B = μ₀(H + M), after `as_quantity`).
    pub struct Magnetization: kind MagnetizingField;
    /// Linear current density, A_s: current per width of a sheet, a kind of its own.
    pub struct LinearCurrentDensity = kind [a 1, m -1];
    /// Capacitance, C: the open root of F.
    pub struct Capacitance = [kg -1, m -2, s 4, a 2];
    /// Inductance, L: the open root of H.
    pub struct Inductance = [kg 1, m 2, s -2, a -2];

    /// Plane angle, the open root of rad.
    pub struct PlaneAngle = [rad 1];
    /// Phase angle, φ: the argument of a phasor, an NCO's phase, a motor's electrical angle. Not a mechanical
    /// angle: the named steps `as_phase` and `as_plane_angle` cross.
    pub struct PhaseAngle: kind PlaneAngle;
}
